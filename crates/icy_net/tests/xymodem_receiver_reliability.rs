use icy_net::{
    Connection,
    connection::channel::ChannelConnection,
    crc::get_crc16,
    protocol::{Protocol, TransferState, XYModemVariant, XYmodem},
};
use std::{future::Future, time::Duration};

const SOH: u8 = 1;
const STX: u8 = 2;
const EOT: u8 = 4;
const ACK: u8 = 6;
const NAK: u8 = 21;
const CAN: u8 = 24;

async fn bounded<T>(f: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(2), f).await.expect("receiver stuck")
}

fn packet(num: u8, payload: &[u8], len: usize, pad: u8) -> Vec<u8> {
    let mut data = vec![if len == 128 { SOH } else { STX }, num, !num];
    data.extend_from_slice(payload);
    data.resize(len + 3, pad);
    data.extend_from_slice(&get_crc16(&data[3..]).to_be_bytes());
    data
}

struct Receiver {
    protocol: XYmodem,
    state: TransferState,
    conn: ChannelConnection,
    peer: ChannelConnection,
}
impl Receiver {
    async fn new(variant: XYModemVariant) -> Self {
        let (mut conn, peer) = ChannelConnection::create_pair();
        let mut protocol = XYmodem::new(variant);
        let state = protocol.initiate_recv(&mut conn).await.unwrap();
        let mut r = Self { protocol, state, conn, peer };
        r.byte().await;
        r
    }
    async fn byte(&mut self) -> u8 {
        bounded(self.peer.read_u8()).await.unwrap()
    }
    async fn step(&mut self) -> icy_net::Result<()> {
        bounded(self.protocol.update_transfer(&mut self.conn, &mut self.state)).await
    }
    async fn block(&mut self, bytes: &[u8]) -> icy_net::Result<()> {
        self.peer.send(bytes).await.unwrap();
        self.step().await?;
        self.step().await
    }
    async fn offer(&mut self, info: &[u8], len: usize) {
        self.block(&packet(0, info, len, 0)).await.unwrap();
        assert_eq!(self.byte().await, ACK);
        assert_eq!(self.byte().await, b'C');
    }
    async fn finish(&mut self) {
        self.peer.send(&[EOT]).await.unwrap();
        self.step().await.unwrap();
        assert_eq!(self.byte().await, NAK);
        self.peer.send(&[EOT]).await.unwrap();
        self.step().await.unwrap();
        assert_eq!(self.byte().await, ACK);
        assert_eq!(self.byte().await, b'C');
    }
    fn content(&self) -> Vec<u8> {
        std::fs::read(&self.state.recieve_state.finished_files.last().unwrap().1).unwrap()
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        for (_, path) in &self.state.recieve_state.finished_files {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[tokio::test]
async fn malformed_header_metadata_never_panics_or_silently_becomes_unknown_size() {
    for info in [vec![b'a'; 128], vec![0xff; 128], b"bad\0-1\0".to_vec(), b"bad\018446744073709551616\0".to_vec()] {
        let mut r = Receiver::new(XYModemVariant::YModem).await;
        assert!(r.block(&packet(0, &info, 128, 0)).await.is_err());
        assert!(r.state.recieve_state.finished_files.is_empty());
    }
}

#[tokio::test]
async fn eight_bit_and_utf8_metadata_use_raw_terminator_offsets() {
    for (info, name) in [(b"\xff\0".as_slice(), "ÿ"), ("ä.txt\03\0".as_bytes(), "ä.txt")] {
        let mut r = Receiver::new(XYModemVariant::YModem).await;
        r.offer(info, 128).await;
        assert_eq!(r.state.recieve_state.file_name, name);
        if name == "ä.txt" {
            assert_eq!(r.state.recieve_state.file_size, 3);
        }
    }
}

#[tokio::test]
async fn stx_block_zero_is_a_file_header_not_data() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    let name = "n".repeat(180);
    let info = format!("{name}\0{}\0", 3);
    r.offer(info.as_bytes(), 1024).await;
    assert_eq!(r.state.recieve_state.file_name, name);
    r.block(&packet(1, b"abc", 128, 26)).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    r.finish().await;
    assert_eq!(r.content(), b"abc");
}

#[tokio::test]
async fn repeated_header_reissues_ack_and_data_request_without_resetting_file() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    let hdr = packet(0, b"file\03\0", 128, 0);
    r.offer(b"file\03\0", 128).await;
    r.block(&hdr).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    assert_eq!(r.byte().await, b'C');
    r.block(&packet(1, b"abc", 128, 26)).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    r.finish().await;
    assert_eq!(r.content(), b"abc");
}

#[tokio::test]
async fn zero_length_is_distinct_from_omitted_length() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    r.offer(b"empty\00\0", 128).await;
    assert!(
        r.block(&packet(1, b"unexpected", 128, 26)).await.is_err(),
        "explicit zero must not become unknown length"
    );
    assert!(r.state.recieve_state.finished_files.is_empty());
}

#[tokio::test]
async fn received_byte_accounting_excludes_padding_and_duplicate_blocks() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    r.offer(b"file\03\0", 128).await;
    let data = packet(1, b"ab\x1a", 128, 26);
    r.block(&data).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    r.block(&data).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    r.finish().await;
    assert_eq!(r.content(), b"ab\x1a");
    assert_eq!(r.state.recieve_state.total_bytes_transfered, 3);
}

#[tokio::test]
async fn xmodem_empty_file_is_accepted_without_a_data_block() {
    for variant in [XYModemVariant::XModemCRC, XYModemVariant::XModem1kG] {
        let mut r = Receiver::new(variant).await;
        r.peer.send(&[EOT]).await.unwrap();
        r.step().await.unwrap();
        assert_eq!(r.byte().await, ACK);
        assert!(r.state.is_finished);
        assert_eq!(r.content(), b"");
        assert_eq!(r.peer.try_read(&mut [0]).await.unwrap(), 0);
    }
}

#[tokio::test]
async fn xmodem_g_finishes_single_file_without_requesting_a_batch_header() {
    let mut r = Receiver::new(XYModemVariant::XModem1kG).await;
    r.block(&packet(1, b"abc", 128, 26)).await.unwrap();
    r.peer.send(&[EOT]).await.unwrap();
    r.step().await.unwrap();
    assert_eq!(r.byte().await, ACK);
    assert!(r.state.is_finished);
    assert_eq!(r.peer.try_read(&mut [0]).await.unwrap(), 0);
    assert_eq!(r.content(), b"abc");
}

#[tokio::test]
async fn double_can_cancels_before_or_during_a_file() {
    for started in [false, true] {
        let mut r = Receiver::new(XYModemVariant::YModem).await;
        if started {
            r.offer(b"file\0100\0", 128).await;
        }
        r.peer.send(&[CAN, CAN]).await.unwrap();
        assert!(r.step().await.is_err());
        assert!(r.state.is_finished);
        assert!(r.state.recieve_state.finished_files.is_empty());
    }
}

#[tokio::test]
async fn repeated_eot_after_lost_ack_does_not_duplicate_a_finished_file() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    r.offer(b"file\00\0", 128).await;
    r.finish().await;
    r.peer.send(&[EOT]).await.unwrap();
    r.step().await.unwrap();
    assert_eq!(r.byte().await, ACK);
    assert_eq!(r.byte().await, b'C');
    assert_eq!(r.state.recieve_state.finished_files.len(), 1);
    r.block(&packet(0, b"", 128, 0)).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    assert!(r.state.is_finished);
}

#[tokio::test]
async fn xmodem_does_not_ack_a_phantom_initial_block_zero() {
    let mut r = Receiver::new(XYModemVariant::XModemCRC).await;
    assert!(r.block(&packet(0, b"bad", 128, 26)).await.is_err());
    assert!(r.state.recieve_state.finished_files.is_empty());
}

#[tokio::test]
async fn mixed_blocks_and_sequence_rollover_preserve_data_and_duplicates() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    let mut expected = Vec::new();
    for index in 0..260 {
        expected.extend(vec![(index % 251) as u8; if index % 2 == 0 { 128 } else { 1024 }]);
    }
    r.offer(format!("rollover\0{}\0", expected.len()).as_bytes(), 128).await;
    for index in 0..260 {
        let len = if index % 2 == 0 { 128 } else { 1024 };
        let data = packet((index + 1) as u8, &vec![(index % 251) as u8; len], len, 26);
        r.block(&data).await.unwrap();
        assert_eq!(r.byte().await, ACK);
        if index >= 254 {
            r.block(&data).await.unwrap();
            assert_eq!(r.byte().await, ACK);
        }
    }
    r.finish().await;
    assert_eq!(r.content(), expected);
    assert_eq!(r.state.recieve_state.total_bytes_transfered, expected.len() as u64);
}

#[tokio::test]
async fn incomplete_or_overrun_files_never_finish() {
    for size in [3, 129] {
        let mut r = Receiver::new(XYModemVariant::YModem).await;
        r.offer(format!("file\0{size}\0").as_bytes(), 128).await;
        r.block(&packet(1, &[42; 128], 128, 26)).await.unwrap();
        assert_eq!(r.byte().await, ACK);
        let result = if size == 3 {
            r.block(&packet(2, b"overflow", 128, 26)).await
        } else {
            r.peer.send(&[EOT]).await.unwrap();
            r.step().await
        };
        assert!(result.is_err());
        assert!(r.state.is_finished);
        assert!(r.state.recieve_state.finished_files.is_empty());
        assert_eq!(r.byte().await, CAN);
    }
}

#[tokio::test]
async fn corrupt_header_and_data_retries_are_bounded() {
    for header in [true, false] {
        for complement in [true, false] {
            let mut r = Receiver::new(XYModemVariant::YModem).await;
            if !header {
                r.offer(b"file\03\0", 128).await;
            }
            let mut bad = packet(if header { 0 } else { 1 }, b"file\03\0", 128, 0);
            if complement {
                bad[2] ^= 1;
            } else {
                *bad.last_mut().unwrap() ^= 1;
            }
            let mut failed = false;
            for _ in 0..12 {
                let result = r.block(&bad).await;
                if result.is_err() {
                    assert_eq!(r.byte().await, CAN);
                    failed = true;
                    break;
                }
                assert_eq!(r.byte().await, NAK);
            }
            assert!(failed, "corrupt blocks retried forever");
            assert!(r.state.recieve_state.finished_files.is_empty());
        }
    }
}

#[tokio::test]
async fn crc_error_budget_resets_after_successful_data() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    r.offer(b"file\0256\0", 128).await;
    for num in [1, 2] {
        let good = packet(num, &[42; 128], 128, 26);
        let mut bad = good.clone();
        *bad.last_mut().unwrap() ^= 1;
        for _ in 0..6 {
            r.block(&bad).await.unwrap();
            assert_eq!(r.byte().await, NAK);
        }
        r.block(&good).await.unwrap();
        assert_eq!(r.byte().await, ACK);
    }
    r.finish().await;
    assert_eq!(r.content(), vec![42; 256]);
}

#[tokio::test]
async fn real_sender_receiver_matrix_all_variants_and_block_edges() {
    use std::io::Write;
    for variant in [
        XYModemVariant::XModem,
        XYModemVariant::XModemCRC,
        XYModemVariant::XModem1k,
        XYModemVariant::XModem1kG,
        XYModemVariant::YModem,
        XYModemVariant::YModemG,
    ] {
        for len in [0, 1, 127, 128, 129, 1023, 1024, 1025, 33001] {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            let data: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
            file.write_all(&data).unwrap();
            let (mut send_conn, mut recv_conn) = ChannelConnection::create_pair();
            let mut sender = XYmodem::new(variant);
            let mut receiver = XYmodem::new(variant);
            let mut send = sender.initiate_send(&mut send_conn, &[file.path().to_path_buf()]).await.unwrap();
            let mut recv = receiver.initiate_recv(&mut recv_conn).await.unwrap();
            bounded(async {
                tokio::join!(
                    async {
                        while !send.is_finished {
                            sender.update_transfer(&mut send_conn, &mut send).await.unwrap();
                            tokio::task::yield_now().await;
                        }
                    },
                    async {
                        while !recv.is_finished {
                            receiver.update_transfer(&mut recv_conn, &mut recv).await.unwrap();
                            tokio::task::yield_now().await;
                        }
                    }
                );
            })
            .await;
            assert_eq!(send.send_state.finished_files.len(), 1, "{variant:?} size {len}");
            assert_eq!(recv.recieve_state.finished_files.len(), 1, "{variant:?} size {len}");
            let path = tempfile::TempPath::try_from_path(&recv.recieve_state.finished_files[0].1).unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), data, "{variant:?} size {len}");
            assert_eq!(recv.recieve_state.total_bytes_transfered, len as u64);
        }
    }
}

#[tokio::test]
async fn missing_initial_data_request_is_repeated_in_negotiated_mode() {
    let mut r = Receiver::new(XYModemVariant::YModem).await;
    r.offer(b"file\03\0", 128).await;
    tokio::time::timeout(Duration::from_secs(4), r.protocol.update_transfer(&mut r.conn, &mut r.state))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.byte().await, b'C');
    r.block(&packet(1, b"abc", 128, 26)).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    r.finish().await;
    assert_eq!(r.content(), b"abc");
}

#[tokio::test]
async fn local_cancel_stops_both_directions_and_protocol_can_be_reused() {
    use std::io::Write;
    let mut r = Receiver::new(XYModemVariant::XModemCRC).await;
    r.protocol.cancel_transfer(&mut r.conn).await.unwrap();
    r.step().await.unwrap();
    assert!(r.state.is_finished);
    let mut cancel = [0; 6];
    r.peer.read_exact(&mut cancel).await.unwrap();
    assert_eq!(cancel, [CAN; 6]);

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"abc").unwrap();
    r.state = r.protocol.initiate_send(&mut r.conn, &[file.path().to_path_buf()]).await.unwrap();
    r.peer.send(b"C").await.unwrap();
    r.step().await.unwrap();
    r.step().await.unwrap();
    let mut data = [0; 133];
    r.peer.read_exact(&mut data).await.unwrap();
    assert_eq!(&data[..6], &[SOH, 1, 254, b'a', b'b', b'c']);
    r.protocol.cancel_transfer(&mut r.conn).await.unwrap();
    r.step().await.unwrap();
    assert!(r.state.is_finished);
    r.peer.read_exact(&mut cancel).await.unwrap();
    assert_eq!(cancel, [CAN; 6]);
    assert!(r.state.send_state.finished_files.is_empty());

    r.state = r.protocol.initiate_recv(&mut r.conn).await.unwrap();
    assert_eq!(r.byte().await, b'C');
    r.block(&packet(1, b"abc", 128, 26)).await.unwrap();
    assert_eq!(r.byte().await, ACK);
    r.peer.send(&[EOT]).await.unwrap();
    r.step().await.unwrap();
    assert_eq!(r.byte().await, ACK);
    assert!(r.state.is_finished);
    assert_eq!(r.content(), b"abc");
}
