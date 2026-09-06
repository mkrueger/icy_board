use std::{collections::VecDeque, future::Future, io::Write, path::PathBuf, time::Duration};

use async_trait::async_trait;
use icy_net::{
    Connection, ConnectionType,
    protocol::{Protocol, TransferState, XYModemVariant, XYmodem},
};
use tempfile::NamedTempFile;

const SOH: u8 = 1;
const STX: u8 = 2;
const EOT: u8 = 4;
const ACK: u8 = 6;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;

enum Input {
    Byte(u8),
    // Exercise the sender's actual timer, not an injected lookalike error.
    Silence,
    Closed,
}

#[derive(Default)]
struct Wire {
    input: VecDeque<Input>,
    sent: Vec<Vec<u8>>,
}

#[async_trait]
impl Connection for Wire {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        match self.input.pop_front() {
            Some(Input::Byte(byte)) => {
                buf[0] = byte;
                Ok(1)
            }
            Some(Input::Closed) => Ok(0),
            Some(Input::Silence) | None => std::future::pending().await,
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        if matches!(self.input.front(), Some(Input::Byte(_))) {
            self.read(buf).await
        } else {
            Ok(0)
        }
    }

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.sent.push(buf.to_vec());
        Ok(())
    }
}

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("sender update exceeded deadline")
}

fn file(data: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    file
}

struct Sender {
    protocol: XYmodem,
    state: TransferState,
    wire: Wire,
}

impl Sender {
    async fn new(variant: XYModemVariant, files: &[PathBuf], mode: u8) -> Self {
        let mut protocol = XYmodem::new(variant);
        let mut wire = Wire::default();
        let state = protocol.initiate_send(&mut wire, files).await.unwrap();
        let mut sender = Self { protocol, state, wire };
        sender.reply(&[mode]);
        sender.step().await;
        sender
    }

    fn reply(&mut self, bytes: &[u8]) {
        self.wire.input.extend(bytes.iter().copied().map(Input::Byte));
    }

    async fn result(&mut self) -> icy_net::Result<()> {
        bounded(self.protocol.update_transfer(&mut self.wire, &mut self.state)).await
    }

    async fn step(&mut self) {
        self.result().await.unwrap();
    }

    async fn accept_header(&mut self, request: u8) {
        self.step().await;
        self.reply(&[ACK, request]);
        self.step().await;
    }

    async fn assert_stopped_without_file(&mut self) {
        assert!(self.state.send_state.finished_files.is_empty());
        let count = self.wire.sent.len();
        self.step().await;
        assert!(self.state.is_finished);
        assert_eq!(self.wire.sent.len(), count, "failed sender must not resume or finish a file");
    }
}

// Independent CRC/checksum and complete block layout assertions, including padding.
fn assert_block(wire: &[u8], number: u8, data: &[u8], pad: u8, crc: bool) {
    let len = if data.len() <= 128 { 128 } else { 1024 };
    assert_eq!(&wire[..3], &[if len == 128 { SOH } else { STX }, number, !number]);
    assert_eq!(wire.len(), 3 + len + if crc { 2 } else { 1 });
    assert_eq!(&wire[3..3 + data.len()], data);
    assert!(wire[3 + data.len()..3 + len].iter().all(|&byte| byte == pad));
    if crc {
        let mut check = 0u16;
        for &byte in &wire[3..3 + len] {
            check ^= (byte as u16) << 8;
            for _ in 0..8 {
                check = if check & 0x8000 != 0 { (check << 1) ^ 0x1021 } else { check << 1 };
            }
        }
        assert_eq!(&wire[3 + len..], &check.to_be_bytes());
    } else {
        assert_eq!(wire[3 + len], wire[3..3 + len].iter().fold(0u8, |sum, &b| sum.wrapping_add(b)));
    }
}

#[tokio::test]
async fn data_timeout_retransmits_identical_block_without_counting_bytes_twice() {
    let data = vec![0x5a; 256];
    let file = file(&data);
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    s.wire.input.push_back(Input::Silence);
    s.step().await;
    assert_eq!(s.state.send_state.errors, 1);
    s.step().await;
    assert_eq!(s.wire.sent[0], s.wire.sent[1]);
    assert_eq!(s.state.send_state.total_bytes_transfered, 128);
    s.reply(&[ACK]);
    s.step().await;
    s.step().await;
    assert_block(&s.wire.sent[2], 2, &data[128..], 0x1a, true);
    s.reply(&[ACK, ACK]);
    s.step().await;
    assert_eq!(s.wire.sent.last().unwrap(), &[EOT]);
    assert_eq!(s.state.send_state.total_bytes_transfered, 256);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
}

#[tokio::test]
async fn header_timeout_retransmits_identical_header_then_starts_at_block_one() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], b'C').await;
    s.step().await;
    s.wire.input.push_back(Input::Silence);
    s.step().await;
    s.step().await;
    assert_eq!(s.wire.sent[0], s.wire.sent[1]);
    assert_eq!(s.state.send_state.errors, 1);
    assert_eq!(s.state.send_state.total_bytes_transfered, 0);
    s.reply(&[ACK, b'C']);
    s.step().await;
    s.step().await;
    assert_block(&s.wire.sent[2], 1, b"abc", 0x1a, true);
}

#[tokio::test]
async fn header_nak_retry_preserves_next_data_block_number() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], b'C').await;
    s.step().await;
    s.reply(&[NAK]);
    s.step().await;
    s.step().await;
    assert_eq!(s.wire.sent[0], s.wire.sent[1]);
    s.reply(&[ACK, b'C']);
    s.step().await;
    s.step().await;
    assert_block(&s.wire.sent[2], 1, b"abc", 0x1a, true);
}

#[tokio::test]
async fn terminal_header_timeout_is_retried() {
    let mut s = Sender::new(XYModemVariant::YModem, &[], b'C').await;
    s.step().await;
    s.wire.input.push_back(Input::Silence);
    s.step().await;
    s.step().await;
    assert_eq!(s.wire.sent[0], s.wire.sent[1]);
    assert_block(&s.wire.sent[1], 0, &[0], 0, true);
    s.reply(&[ACK]);
    s.step().await;
    assert!(s.state.is_finished);
    assert!(s.state.send_state.finished_files.is_empty());
}

#[tokio::test]
async fn checksum_ymodem_batch_accepts_nak_requests_and_direct_eot_ack() {
    let first = file(b"one");
    let second = file(b"two");
    let mut s = Sender::new(XYModemVariant::YModem, &[first.path().into(), second.path().into()], NAK).await;
    for (index, file) in [&first, &second].into_iter().enumerate() {
        s.accept_header(NAK).await;
        let mut header = file.path().file_name().unwrap().as_encoded_bytes().to_vec();
        header.extend_from_slice(b"\0");
        header.extend_from_slice(b"3");
        assert_block(s.wire.sent.last().unwrap(), 0, &header, 0, false);
        s.step().await;
        assert_block(s.wire.sent.last().unwrap(), 1, if index == 0 { b"one" } else { b"two" }, 0x1a, false);
        s.reply(&[ACK, ACK]);
        s.step().await;
        assert_eq!(s.wire.sent.last().unwrap(), &[EOT]);
        assert_eq!(s.state.send_state.finished_files.len(), index + 1);
        s.reply(&[NAK]);
        s.step().await;
    }
    s.step().await;
    assert_block(s.wire.sent.last().unwrap(), 0, &[0], 0, false);
    s.reply(&[ACK]);
    s.step().await;
    assert!(s.state.is_finished);
    assert_eq!(s.state.send_state.total_bytes_transfered, 6);
    assert_eq!(s.state.send_state.errors, 0);
}

#[tokio::test]
async fn classic_ymodem_accepts_direct_eot_ack() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], b'C').await;
    s.accept_header(b'C').await;
    s.step().await;
    s.reply(&[ACK, ACK]);
    s.step().await;
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    assert_eq!(s.wire.sent.last().unwrap(), &[EOT]);
}

#[tokio::test]
async fn xmodem_eot_retries_nak_and_timeout_until_ack() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    s.reply(&[ACK, NAK]);
    s.wire.input.push_back(Input::Silence);
    s.reply(&[ACK]);
    s.step().await;
    assert_eq!(&s.wire.sent[1..], &[vec![EOT], vec![EOT], vec![EOT]]);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    assert_eq!(s.state.send_state.errors, 2);
}

#[tokio::test]
async fn eot_retry_budget_is_bounded_and_does_not_finish_file() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    s.reply(&[ACK]);
    s.reply(&[NAK; 20]);
    assert!(s.result().await.is_err());
    assert_eq!(s.wire.sent.iter().filter(|b| b.as_slice() == [EOT]).count(), 10);
    assert_eq!(s.state.send_state.errors, 10);
    assert_eq!(s.wire.sent.last().unwrap(), &[CAN; 6]);
    s.assert_stopped_without_file().await;
}

#[tokio::test]
async fn data_nak_budget_counts_each_failure_once_and_reuses_packet() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    let original = s.wire.sent[0].clone();
    for attempt in 0..6 {
        s.reply(&[NAK]);
        let result = s.result().await;
        assert_eq!(s.state.send_state.errors, attempt + 1);
        if attempt == 5 {
            assert!(result.is_err());
        } else {
            result.unwrap();
            s.step().await;
            assert_eq!(s.wire.sent.last().unwrap(), &original);
        }
    }
    assert_eq!(s.state.send_state.total_bytes_transfered, 3);
    s.assert_stopped_without_file().await;
}

#[tokio::test]
async fn streaming_cancel_is_polled_mid_file_and_across_updates() {
    for (variant, split) in [
        (XYModemVariant::XModem1kG, false),
        (XYModemVariant::XModem1kG, true),
        (XYModemVariant::YModemG, false),
        (XYModemVariant::YModemG, true),
    ] {
        let file = file(&vec![0x42; 8192]);
        let mut s = Sender::new(variant, &[file.path().into()], b'G').await;
        if variant == XYModemVariant::YModemG {
            s.step().await;
            s.reply(b"G");
            s.step().await;
        }
        s.step().await;
        assert_block(s.wire.sent.last().unwrap(), 1, &[0x42; 1024], 0x1a, true);
        if split {
            s.reply(&[CAN]);
            s.step().await;
            assert_block(s.wire.sent.last().unwrap(), 2, &[0x42; 1024], 0x1a, true);
            s.reply(&[CAN]);
        } else {
            s.reply(&[CAN, CAN]);
        }
        assert_eq!(s.result().await.unwrap_err().to_string(), "transmission canceled");
        assert_eq!(s.state.send_state.total_bytes_transfered, if split { 2048 } else { 1024 });
        assert!(!s.wire.sent.iter().any(|b| b.as_slice() == [EOT]));
        s.assert_stopped_without_file().await;
    }
}

#[tokio::test]
async fn eot_cancel_never_finishes_file() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    s.reply(&[ACK, CAN, CAN]);
    assert_eq!(s.result().await.unwrap_err().to_string(), "transmission canceled");
    s.assert_stopped_without_file().await;
}

#[tokio::test]
async fn shrinking_source_aborts_instead_of_spinning_or_completing() {
    for variant in [XYModemVariant::XModemCRC, XYModemVariant::YModem] {
        let file = file(&[0x42; 256]);
        let mut s = Sender::new(variant, &[file.path().into()], b'C').await;
        if variant == XYModemVariant::YModem {
            s.accept_header(b'C').await;
        }
        file.as_file().set_len(0).unwrap();
        assert!(s.result().await.unwrap_err().to_string().contains("file is incomplete"));
        assert!(!s.wire.sent.iter().any(|b| b.as_slice() == [EOT]));
        s.assert_stopped_without_file().await;
    }
}

#[tokio::test]
async fn block_number_rollover_and_nak_keep_payload_and_accounting() {
    let data: Vec<u8> = (0..257).flat_map(|n| vec![n as u8; 128]).collect();
    let file = file(&data);
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    for index in 0..257 {
        s.step().await;
        assert_block(
            s.wire.sent.last().unwrap(),
            (index + 1) as u8,
            &data[index * 128..(index + 1) * 128],
            0x1a,
            true,
        );
        if index == 255 {
            let original = s.wire.sent.last().unwrap().clone();
            s.reply(&[NAK]);
            s.step().await;
            s.step().await;
            assert_eq!(s.wire.sent.last().unwrap(), &original);
        }
        s.reply(&[ACK]);
        if index == 256 {
            s.reply(&[ACK]);
        }
        s.step().await;
    }
    assert_eq!(s.state.send_state.total_bytes_transfered, data.len() as u64);
    assert_eq!(s.state.send_state.errors, 1);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
}

#[tokio::test]
async fn closed_connection_is_fatal_not_a_retransmission_timeout() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    s.wire.input.push_back(Input::Closed);
    assert!(s.result().await.is_err());
    s.assert_stopped_without_file().await;
}

#[tokio::test]
async fn header_retry_budget_is_bounded_and_does_not_advance_the_batch() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], b'C').await;
    s.step().await;
    let original = s.wire.sent[0].clone();
    for attempt in 0..6 {
        s.reply(&[NAK]);
        let result = s.result().await;
        assert_eq!(s.state.send_state.errors, attempt + 1);
        if attempt == 5 {
            assert!(result.is_err());
        } else {
            result.unwrap();
            s.step().await;
            assert_eq!(s.wire.sent.last().unwrap(), &original);
        }
    }
    assert_eq!(s.state.send_state.total_bytes_transfered, 0);
    assert_eq!(s.wire.sent.last().unwrap(), &[CAN; 6]);
    s.assert_stopped_without_file().await;
}

#[tokio::test]
async fn lost_data_and_next_file_requests_wait_without_resending_acknowledged_data() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], NAK).await;
    s.step().await;
    s.reply(&[ACK]);
    s.wire.input.push_back(Input::Silence);
    s.step().await;
    assert_eq!(s.wire.sent.len(), 1, "ACK alone must not start data");
    assert_eq!(s.state.send_state.errors, 1);
    s.reply(&[NAK]);
    s.step().await;
    s.step().await;
    assert_block(&s.wire.sent[1], 1, b"abc", 0x1a, false);
    s.reply(&[ACK, NAK, ACK]);
    s.step().await;
    assert_eq!(&s.wire.sent[2..], &[vec![EOT], vec![EOT]]);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    s.wire.input.push_back(Input::Silence);
    s.step().await;
    assert_eq!(s.wire.sent.len(), 4);
    assert_eq!(s.state.send_state.errors, 2, "normal YMODEM EOT NAK is not an error");
    s.reply(&[NAK]);
    s.step().await;
    s.step().await;
    assert_block(&s.wire.sent[4], 0, &[0], 0, false);
    s.reply(&[ACK]);
    s.step().await;
    assert!(s.state.is_finished);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    assert_eq!(s.state.send_state.total_bytes_transfered, 3);
}

#[tokio::test]
async fn repeated_stray_acks_do_not_keep_request_wait_alive_forever() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], b'C').await;
    s.step().await;
    s.reply(&[ACK, ACK]);
    s.step().await;
    for attempt in 1..6 {
        s.reply(&[ACK]);
        let result = s.result().await;
        if attempt == 5 {
            assert!(result.is_err());
        } else {
            result.unwrap();
        }
    }
    assert_eq!(s.state.send_state.errors, 6);
    assert_eq!(s.state.send_state.total_bytes_transfered, 0);
    s.assert_stopped_without_file().await;
}

#[tokio::test]
async fn a_single_can_before_ack_is_line_noise_not_cancellation() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::XModemCRC, &[file.path().into()], b'C').await;
    s.step().await;
    s.reply(&[CAN, ACK, ACK]);
    s.step().await;
    assert!(s.state.is_finished);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    assert_eq!(s.state.send_state.errors, 0);
}

#[tokio::test]
async fn partial_source_block_is_not_padded_and_reported_complete() {
    for variant in [XYModemVariant::XModemCRC, XYModemVariant::YModem] {
        let file = file(&[0x42; 256]);
        let mut s = Sender::new(variant, &[file.path().into()], b'C').await;
        if variant == XYModemVariant::YModem {
            s.accept_header(b'C').await;
        }
        file.as_file().set_len(129).unwrap();
        if variant == XYModemVariant::XModemCRC {
            s.step().await;
            assert_block(s.wire.sent.last().unwrap(), 1, &[0x42; 128], 0x1a, true);
            s.reply(&[ACK]);
            s.step().await;
        }
        assert_eq!(
            s.result().await.unwrap_err().to_string(),
            "file is incomplete: expected 256 bytes but received 129"
        );
        assert_eq!(s.wire.sent.last().unwrap(), &[CAN; 6]);
        s.assert_stopped_without_file().await;
    }
}

#[tokio::test]
async fn a_long_header_uses_stx_without_truncation() {
    let dir = tempfile::tempdir().unwrap();
    let name = "a".repeat(200);
    let path = dir.path().join(&name);
    std::fs::write(&path, b"abc").unwrap();
    let mut s = Sender::new(XYModemVariant::YModem, &[path], b'C').await;
    s.step().await;
    let mut data = name.into_bytes();
    data.extend_from_slice(b"\0");
    data.extend_from_slice(b"3");
    assert_block(&s.wire.sent[0], 0, &data, 0, true);
}

#[tokio::test]
async fn crc_next_file_nak_repeats_eot_without_finishing_file_twice() {
    let file = file(b"abc");
    let mut s = Sender::new(XYModemVariant::YModem, &[file.path().into()], b'C').await;
    s.accept_header(b'C').await;
    s.step().await;
    s.reply(&[ACK, NAK, ACK]);
    s.step().await;
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    s.reply(&[NAK, ACK]);
    s.step().await;
    assert_eq!(&s.wire.sent[2..], &[vec![EOT], vec![EOT], vec![EOT]]);
    s.reply(b"C");
    s.step().await;
    s.step().await;
    assert_block(s.wire.sent.last().unwrap(), 0, &[0], 0, true);
    s.reply(&[ACK]);
    s.step().await;
    assert!(s.state.is_finished);
    assert_eq!(s.state.send_state.finished_files.len(), 1);
    assert_eq!(s.state.send_state.total_bytes_transfered, 3);
}
