use std::{collections::VecDeque, io::Write, time::Duration};

use async_trait::async_trait;
use icy_net::{
    Connection, ConnectionType,
    connection::channel::ChannelConnection,
    protocol::{Kermit, Protocol, TransferProtocolType, TransferState},
};
use tempfile::NamedTempFile;

const INIT: &[u8] = b"~% @-#&1~";

#[derive(Default)]
struct Wire {
    input: VecDeque<u8>,
    sent: Vec<Vec<u8>>,
    closed: bool,
}

#[async_trait]
impl Connection for Wire {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        if let Some(byte) = self.input.pop_front() {
            buf[0] = byte;
            Ok(1)
        } else if self.closed {
            Ok(0)
        } else {
            std::future::pending().await
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        if self.input.is_empty() { Ok(0) } else { self.read(buf).await }
    }

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.sent.push(buf.to_vec());
        Ok(())
    }

    fn unread(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        for &byte in buf.iter().rev() {
            self.input.push_front(byte);
        }
        Ok(())
    }
}

// Independent wire encoder: tests never use the implementation's packet codec.
fn packet(sequence: u8, kind: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![1, 32 + data.len() as u8 + 3, 32 + sequence, kind];
    bytes.extend(data);
    let sum: usize = bytes[1..].iter().map(|b| *b as usize).sum();
    bytes.push(32 + ((sum + (sum / 64 % 4)) % 64) as u8);
    bytes.push(13);
    bytes
}

fn quote(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    for &original in data {
        let byte = original & 127;
        if original > 127 {
            result.push(b'&');
        }
        match byte {
            0..=31 | 127 => result.extend([b'#', byte ^ 64]),
            b'#' | b'&' | b'~' => result.extend([b'#', byte]),
            _ => result.push(byte),
        }
    }
    result
}

struct Receiver {
    protocol: Kermit,
    state: TransferState,
    wire: Wire,
}

impl Receiver {
    async fn new() -> Self {
        let mut protocol = Kermit::new();
        let mut wire = Wire::default();
        let state = protocol.initiate_recv(&mut wire).await.unwrap();
        Self { protocol, state, wire }
    }

    async fn feed(&mut self, sequence: u8, kind: u8, data: &[u8]) {
        self.wire.input.extend(packet(sequence, kind, data));
        self.protocol.update_transfer(&mut self.wire, &mut self.state).await.unwrap();
    }

    fn take_file(&mut self) -> Vec<u8> {
        let (_, path) = self.state.recieve_state.finished_files.pop().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        bytes
    }
}

#[test]
fn configuration_and_known_ack_vector() {
    assert_eq!(packet(0, b'Y', b""), b"\x01# Y>\r");
    let protocol: TransferProtocolType = serde_json::from_str("\"@KeRmIt\"").unwrap();
    assert_eq!(protocol, TransferProtocolType::Kermit);
    assert_eq!(serde_json::to_string(&protocol).unwrap(), "\"@kermit\"");
}

#[tokio::test(start_paused = true)]
async fn receives_quoted_binary_duplicates_and_repeat_counts() {
    let mut r = Receiver::new().await;
    r.feed(0, b'S', INIT).await;
    assert_eq!(r.wire.sent[0], packet(0, b'Y', INIT));
    r.feed(0, b'S', INIT).await;
    r.feed(1, b'F', b"..\\../binary.bin").await;
    assert_eq!(r.state.recieve_state.file_name, "binary.bin");
    let bytes: Vec<u8> = (0..=255).collect();
    let mut sequence = 2;
    for block in bytes.chunks(25) {
        let encoded = quote(block);
        r.feed(sequence, b'D', &encoded).await;
        r.feed(sequence, b'D', &encoded).await;
        sequence += 1;
    }
    r.feed(sequence, b'D', b"~$&#@").await;
    sequence += 1;
    r.feed(sequence, b'Z', b"").await;
    r.feed(sequence, b'Z', b"").await;
    let mut expected = bytes;
    expected.extend([128; 4]);
    assert_eq!(r.state.recieve_state.total_bytes_transfered, expected.len() as u64);
    assert_eq!(r.take_file(), expected);
    r.feed(sequence + 1, b'B', b"").await;
    r.feed(sequence + 1, b'B', b"").await;
    assert_eq!(r.wire.sent.last().unwrap(), &packet(sequence + 1, b'Y', b""));
    r.wire.input.extend(b"menu");
    r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    assert!(r.state.is_finished);
    assert_eq!(r.wire.input.iter().copied().collect::<Vec<_>>(), b"menu");
}

#[tokio::test(start_paused = true)]
async fn damaged_out_of_order_and_truncated_packets_recover() {
    let mut r = Receiver::new().await;
    r.feed(0, b'S', INIT).await;
    r.feed(1, b'F', b"data.bin").await;
    let mut damaged = packet(2, b'D', b"bad");
    damaged[4] ^= 1;
    r.wire.input.extend(damaged);
    r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    assert_eq!(r.wire.sent.last().unwrap(), &packet(2, b'N', b""));
    r.feed(3, b'D', b"out of order").await;
    assert_eq!(r.wire.sent.last().unwrap(), &packet(2, b'N', b""));
    r.wire.input.extend([1, b'~', b'"']);
    for _ in 0..6 {
        r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    }
    r.feed(2, b'D', b"good").await;
    r.feed(3, b'Z', b"").await;
    assert_eq!(r.take_file(), b"good");
}

#[tokio::test(start_paused = true)]
async fn packet_parser_keeps_partial_input_between_updates() {
    let mut r = Receiver::new().await;
    let init = packet(0, b'S', INIT);
    r.wire.input.extend(&init[..5]);
    r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    assert!(r.wire.sent.is_empty());
    r.wire.input.extend(&init[5..]);
    r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    assert_eq!(r.wire.sent, [packet(0, b'Y', INIT)]);
}

#[tokio::test(start_paused = true)]
async fn default_parameters_allow_unquoted_high_bits_and_custom_control_prefix() {
    for params in [b"".as_slice(), b"     !".as_slice()] {
        let mut r = Receiver::new().await;
        r.feed(0, b'S', params).await;
        r.feed(1, b'F', b"raw.bin").await;
        let control = if params.is_empty() { b'#' } else { b'!' };
        r.feed(2, b'D', &[control, 0xc1, 0xe9, control, b'?', b'&', b'~']).await;
        r.feed(3, b'Z', b"").await;
        assert_eq!(r.take_file(), [0x81, 0xe9, 127, b'&', b'~']);
    }
}

#[tokio::test(start_paused = true)]
async fn receiver_limit_and_remote_discard_do_not_publish_files() {
    let mut r = Receiver::new().await;
    r.protocol.set_receive_limit(0);
    r.feed(0, b'S', INIT).await;
    r.feed(1, b'F', b"skip.bin").await;
    assert_eq!(r.wire.sent.last().unwrap(), &packet(1, b'Y', b"X"));
    r.feed(2, b'D', b"ignored").await;
    assert_eq!(r.wire.sent.last().unwrap(), &packet(2, b'Y', b"X"));
    r.feed(3, b'Z', b"D").await;
    assert!(r.state.recieve_state.finished_files.is_empty());
    assert_eq!(r.state.recieve_state.total_bytes_transfered, 0);
    r.protocol.set_receive_limit(1);
    r.feed(4, b'F', b"cancel.bin").await;
    r.feed(5, b'D', b"not committed").await;
    r.feed(6, b'Z', b"D").await;
    assert!(r.state.recieve_state.finished_files.is_empty());
}

#[tokio::test(start_paused = true)]
async fn remote_error_disconnect_and_cancellation_stop_without_completed_files() {
    for failure in 0..3 {
        let mut r = Receiver::new().await;
        r.feed(0, b'S', INIT).await;
        r.feed(1, b'F', b"partial.bin").await;
        r.feed(2, b'D', b"partial").await;
        match failure {
            0 => r.wire.input.extend(packet(3, b'E', b"Disk full")),
            1 => r.wire.closed = true,
            _ => r.state.request_cancel = true,
        }
        assert!(r.protocol.update_transfer(&mut r.wire, &mut r.state).await.is_err());
        assert!(r.state.is_finished);
        assert!(r.state.recieve_state.finished_files.is_empty());
        assert_eq!(r.state.recieve_state.errors, 1);
        let sent = r.wire.sent.len();
        r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
        assert_eq!(r.wire.sent.len(), sent);
    }
}

#[tokio::test(start_paused = true)]
async fn retry_exhaustion_is_bounded_and_final_ack_grace_expires() {
    let mut r = Receiver::new().await;
    let start = tokio::time::Instant::now();
    loop {
        if r.protocol.update_transfer(&mut r.wire, &mut r.state).await.is_err() {
            break;
        }
        assert!(start.elapsed() <= Duration::from_secs(55));
    }
    assert!(r.state.is_finished);
    assert_eq!(start.elapsed(), Duration::from_secs(55));
    let mut r = Receiver::new().await;
    r.feed(0, b'S', INIT).await;
    r.feed(1, b'B', b"").await;
    let start = tokio::time::Instant::now();
    while !r.state.is_finished {
        r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    }
    assert_eq!(start.elapsed(), Duration::from_secs(11));
}

#[tokio::test(start_paused = true)]
async fn sender_retries_identical_packets_and_counts_only_acknowledged_data() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&[0, 255, b'#', b'&', b'~']).unwrap();
    let mut protocol = Kermit::new();
    let mut wire = Wire::default();
    let mut state = protocol.initiate_send(&mut wire, &[file.path().into()]).await.unwrap();
    assert_eq!(wire.sent[0], packet(0, b'S', INIT));
    for _ in 0..6 {
        protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    }
    assert_eq!(wire.sent[0], wire.sent[1]);
    wire.input.extend(packet(0, b'Y', INIT));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    wire.input.extend(packet(1, b'Y', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    let data = wire.sent.last().unwrap().clone();
    assert_eq!(data, packet(2, b'D', &quote(&[0, 255, b'#', b'&', b'~'])));
    assert_eq!(state.send_state.total_bytes_transfered, 0);
    wire.input.extend(packet(2, b'N', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert_eq!(wire.sent.last().unwrap(), &data);
    // NAK for the next sequence also acknowledges the preceding data.
    wire.input.extend(packet(3, b'N', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert_eq!(wire.sent.last().unwrap(), &packet(3, b'Z', b""));
    assert_eq!(state.send_state.total_bytes_transfered, 5);
    wire.input.extend(packet(2, b'Y', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert!(state.send_state.finished_files.is_empty());
    wire.input.extend(packet(3, b'Y', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert_eq!(state.send_state.finished_files.len(), 1);
    wire.input.extend(packet(4, b'Y', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert!(state.is_finished);
}

#[tokio::test(start_paused = true)]
async fn damaged_break_retransmission_does_not_shorten_final_ack_grace() {
    let mut r = Receiver::new().await;
    r.feed(0, b'S', INIT).await;
    r.feed(1, b'B', b"").await;
    tokio::time::advance(Duration::from_secs(2)).await;
    let mut damaged = packet(1, b'B', b"");
    damaged[4] ^= 1;
    r.wire.input.extend(damaged);
    r.protocol.update_transfer(&mut r.wire, &mut r.state).await.unwrap();
    assert_eq!(r.wire.sent.last().unwrap(), &packet(1, b'Y', b""));
    tokio::time::advance(Duration::from_secs(8)).await;
    let sent = r.wire.sent.len();
    r.feed(1, b'B', b"").await;
    assert!(!r.state.is_finished);
    assert_eq!(r.wire.sent.len(), sent + 1);
    assert_eq!(r.wire.sent.last().unwrap(), &packet(1, b'Y', b""));
}

#[tokio::test(start_paused = true)]
async fn sender_honors_padding_eol_short_packets_and_remote_cancellation() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&[0xff; 1000]).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("a");
    std::fs::copy(file.path(), &path).unwrap();
    let mut protocol = Kermit::new();
    let mut wire = Wire::default();
    let mut state = protocol.initiate_send(&mut wire, &[path]).await.unwrap();
    wire.input.extend(packet(0, b'Y', b",%\"@*#&3~"));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert_eq!(state.send_state.check_size, "Checksum/12");
    let frame = wire.sent.last().unwrap();
    assert_eq!(&frame[..3], &[0, 0, 1]);
    assert_eq!(frame.last(), Some(&10));
    wire.input.extend(packet(1, b'Y', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    let frame = wire.sent.last().unwrap();
    assert!(frame[3] <= b',');
    assert_eq!(&frame[6..frame.len() - 2], b"&#?&#?&#?");
    wire.input.extend(packet(2, b'Y', b"X"));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    let frame = wire.sent.last().unwrap();
    assert_eq!(&frame[5..7], b"ZD");
    wire.input.extend(packet(3, b'Y', b""));
    protocol.update_transfer(&mut wire, &mut state).await.unwrap();
    assert!(state.send_state.finished_files.is_empty());
}

#[tokio::test(start_paused = true)]
async fn binary_batch_round_trip_wraps_sequence_and_preserves_empty_files() {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("binary.bin");
    let empty = directory.path().join("empty.bin");
    let expected: Vec<u8> = (0..16384).map(|n| n as u8).collect();
    std::fs::write(&binary, &expected).unwrap();
    std::fs::write(&empty, []).unwrap();
    let (mut send_wire, mut recv_wire) = ChannelConnection::create_pair();
    let mut sender = TransferProtocolType::Kermit.create();
    let mut receiver = TransferProtocolType::Kermit.create();
    let mut send = sender.initiate_send(&mut send_wire, &[binary, empty]).await.unwrap();
    let mut recv = receiver.initiate_recv(&mut recv_wire).await.unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(
            async {
                while !send.is_finished {
                    sender.update_transfer(&mut send_wire, &mut send).await?;
                }
                send_wire.shutdown().await
            },
            async {
                while !recv.is_finished {
                    receiver.update_transfer(&mut recv_wire, &mut recv).await?;
                }
                icy_net::Result::Ok(())
            }
        )
        .unwrap();
    })
    .await
    .unwrap();
    assert_eq!(send.send_state.finished_files.len(), 2);
    assert_eq!(recv.recieve_state.finished_files.len(), 2);
    assert_eq!(recv.recieve_state.total_bytes_transfered, expected.len() as u64);
    for ((name, path), bytes) in recv.recieve_state.finished_files.iter().zip([expected, Vec::new()]) {
        assert!(matches!(name.as_str(), "binary.bin" | "empty.bin"));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        std::fs::remove_file(path).unwrap();
    }
}
