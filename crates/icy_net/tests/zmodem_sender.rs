use std::{future::Future, io::Write, path::PathBuf, time::Duration};

use async_trait::async_trait;
use icy_net::{
    Connection, ConnectionType,
    protocol::{
        Header, HeaderType, Protocol, TransferState, ZCRCE, ZDLE, ZFrameType, ZPAD, Zmodem,
        zmodem::{constants::ABORT_SEQ, rz::read_subpacket, sz::Sz, zrinit_flag},
    },
};
use tempfile::NamedTempFile;

mod test_connection;
use test_connection::TestConnection;

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future).await.expect("sender regression timed out")
}

fn file(data: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    file
}

fn wire(header: Header) -> Vec<u8> {
    header.build(HeaderType::Hex, false)
}

async fn header(peer: &mut TestConnection) -> Header {
    bounded(Header::read(peer, &mut 0)).await.unwrap().unwrap()
}

async fn assert_idle(peer: &mut TestConnection) {
    assert_eq!(bounded(peer.try_read(&mut [0; 1])).await.unwrap(), 0, "unexpected sender output");
}

struct Sender {
    sz: Sz,
    state: TransferState,
    conn: TestConnection,
    peer: TestConnection,
    crc32: bool,
}

impl Sender {
    async fn new(files: &[PathBuf], crc32: bool) -> Self {
        Self::with_streaming(files, crc32, true).await
    }

    async fn with_streaming(files: &[PathBuf], crc32: bool, streaming: bool) -> Self {
        let (conn, peer) = TestConnection::create_pair();
        let mut sender = Self {
            sz: Sz::new(2),
            state: TransferState::new("sender regression".into()),
            conn,
            peer,
            crc32,
        };
        sender.sz.send(files);
        sender.step().await;
        assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::RQInit);
        let caps = zrinit_flag::CANOVIO | if crc32 { zrinit_flag::CANFC32 } else { 0 };
        let block_size = if streaming { 0 } else { 2 };
        sender.inject(Header::from_flags(ZFrameType::RIinit, block_size, 0, 0, caps)).await;
        sender
    }

    async fn step(&mut self) {
        bounded(self.sz.update_transfer(&mut self.conn, &mut self.state)).await.unwrap();
    }

    async fn inject(&mut self, hdr: Header) {
        bounded(self.peer.send(&wire(hdr))).await.unwrap();
        self.step().await;
    }

    // The response must be sent only after ZFILE: pre-queuing it would exercise
    // the opportunistic poll instead of the sender's blocking handshake read.
    async fn offer_result(&mut self, response: &[u8]) -> icy_net::Result<()> {
        let crc32 = self.crc32;
        let peer = &mut self.peer;
        let (result, ()) = bounded(async {
            tokio::join!(self.sz.update_transfer(&mut self.conn, &mut self.state), async {
                assert_eq!(header(peer).await.frame_type, ZFrameType::File);
                let (_, last, ack) = bounded(read_subpacket(peer, 4096, crc32, false)).await.unwrap();
                assert!(last && ack);
                peer.send(response).await.unwrap();
            })
        })
        .await;
        result
    }

    async fn offer(&mut self, offset: u32) {
        self.offer_result(&wire(Header::from_number(ZFrameType::RPos, offset))).await.unwrap();
    }

    async fn packet(&mut self) -> (Vec<u8>, bool, bool) {
        bounded(read_subpacket(&mut self.peer, 4096, self.crc32, false)).await.unwrap()
    }

    async fn end_batch(&mut self) {
        self.step().await;
        assert_eq!(header(&mut self.peer).await.frame_type, ZFrameType::Fin);
        self.inject(Header::empty(ZFrameType::Fin)).await;
        assert!(self.state.is_finished);
        let mut oo = [0; 2];
        bounded(self.peer.read_exact(&mut oo)).await.unwrap();
        assert_eq!(&oo, b"OO");
        assert_idle(&mut self.peer).await;
    }
}

#[tokio::test]
async fn zrinit_only_confirms_a_file_after_zeof() {
    let file = file(b"abcdef");
    let mut sender = Sender::new(&[file.path().to_path_buf()], true).await;

    // A duplicate initial ZRINIT can also arrive while awaiting ZFILE's reply.
    let mut reply = wire(Header::empty(ZFrameType::RIinit));
    reply.extend(wire(Header::from_number(ZFrameType::RPos, 0)));
    sender.offer_result(&reply).await.unwrap();

    // Before even the ZDATA header, and again before its first subpacket.
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert!(sender.state.send_state.finished_files.is_empty());
    assert!(!sender.sz.transfered_file);
    sender.step().await;
    let data = header(&mut sender.peer).await;
    assert_eq!(data.frame_type, ZFrameType::Data);
    assert_eq!(data.header_type, HeaderType::Bin32, "stale ZRINIT must not change capabilities");
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(sender.state.send_state.cur_bytes_transfered, 0);
    assert!(sender.state.send_state.finished_files.is_empty());
    assert_idle(&mut sender.peer).await;

    sender.step().await;
    assert_eq!(sender.packet().await, (b"ab".to_vec(), false, false));
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(sender.state.send_state.cur_bytes_transfered, 2);
    assert_eq!(sender.state.send_state.file_size, 6);
    assert!(sender.state.send_state.finished_files.is_empty());
    assert!(!sender.sz.transfered_file);
    assert_idle(&mut sender.peer).await;

    sender.step().await;
    assert_eq!(sender.packet().await, (b"cd".to_vec(), false, false));
    sender.step().await;
    assert_eq!(sender.packet().await, (b"ef".to_vec(), true, false));
    let eof = header(&mut sender.peer).await;
    assert_eq!(eof.frame_type, ZFrameType::Eof);
    assert_eq!(eof.number(), 6);
    sender.inject(Header::from_number(ZFrameType::Ack, 6)).await;
    assert!(sender.state.send_state.finished_files.is_empty(), "ZACK is not EOF confirmation");
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(
        sender.state.send_state.finished_files,
        vec![(file.path().file_name().unwrap().to_string_lossy().into_owned(), file.path().to_path_buf())]
    );
    assert!(sender.sz.transfered_file);
    assert_eq!(sender.state.send_state.total_bytes_transfered, 6);
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(sender.state.send_state.finished_files.len(), 1, "duplicate confirmation counted twice");
    sender.end_batch().await;
}

async fn empty_data_frame(crc32: bool, resume: bool) {
    let data: &[u8] = if resume { b"already received" } else { b"" };
    let file = file(data);
    let offset = data.len() as u32;
    let mut sender = Sender::new(&[file.path().to_path_buf()], crc32).await;
    sender.offer(offset).await;
    sender.step().await; // ZDATA
    sender.step().await; // Empty ZCRCE, then ZEOF

    let kind = if crc32 { HeaderType::Bin32 } else { HeaderType::Bin };
    let mut expected = Header::from_number(ZFrameType::Data, offset).build(kind, false);
    let empty = if crc32 {
        Zmodem::encode_subpacket_crc32(ZCRCE, &[], false)
    } else {
        Zmodem::encode_subpacket_crc16(ZCRCE, &[], false)
    };
    assert_eq!(&empty[..2], &[ZDLE, ZCRCE]);
    expected.extend(empty);
    expected.extend(Header::from_number(ZFrameType::Eof, offset).build(kind, false));
    let mut actual = vec![0; expected.len()];
    bounded(sender.peer.read_exact(&mut actual)).await.unwrap();
    assert_eq!(actual, expected, "wire must be ZDATA -> empty ZCRCE/CRC -> ZEOF");
    assert_idle(&mut sender.peer).await;

    // Also decode the actual wire, checking both CRCs and the frame boundary.
    let (mut reader, mut feeder) = TestConnection::create_pair();
    feeder.send(&actual).await.unwrap();
    assert_eq!(header(&mut reader).await.frame_type, ZFrameType::Data);
    assert_eq!(bounded(read_subpacket(&mut reader, 16, crc32, false)).await.unwrap(), (vec![], true, false));
    let eof = header(&mut reader).await;
    assert_eq!(eof.frame_type, ZFrameType::Eof);
    assert_eq!(eof.number(), offset);
    assert!(sender.state.send_state.finished_files.is_empty());
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(sender.state.send_state.finished_files.len(), 1);
    assert_eq!(sender.state.send_state.total_bytes_transfered, 0, "resumed bytes were not transmitted");
    sender.end_batch().await;
}

#[tokio::test]
async fn empty_file_crc16_wire() {
    empty_data_frame(false, false).await;
}

#[tokio::test]
async fn empty_file_crc32_wire() {
    empty_data_frame(true, false).await;
}

#[tokio::test]
async fn resume_at_eof_crc16_wire() {
    empty_data_frame(false, true).await;
}

#[tokio::test]
async fn resume_at_eof_crc32_wire() {
    empty_data_frame(true, true).await;
}

#[tokio::test]
async fn skipped_files_are_not_confirmed_or_counted_in_a_batch() {
    let files = [file(b"skip first"), file(b"ok"), file(b"skip last")];
    let paths: Vec<_> = files.iter().map(|file| file.path().to_path_buf()).collect();
    let mut sender = Sender::new(&paths, true).await;
    sender.offer_result(&wire(Header::empty(ZFrameType::Skip))).await.unwrap();
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert!(sender.state.send_state.finished_files.is_empty());
    assert!(!sender.sz.transfered_file);
    sender.offer(0).await;
    sender.step().await;
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Data);
    sender.step().await;
    assert_eq!(sender.packet().await, (b"ok".to_vec(), true, false));
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Eof);
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    sender.offer_result(&wire(Header::empty(ZFrameType::Skip))).await.unwrap();
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(sender.state.send_state.finished_files.len(), 1);
    assert_eq!(sender.state.send_state.finished_files[0].1, paths[1]);
    assert_eq!(sender.state.send_state.total_bytes_transfered, 2);
    sender.end_batch().await;
}

#[tokio::test]
async fn boundary_flow_control_is_ignored_without_waiting_for_more_input() {
    for blocking in [false, true] {
        let (mut conn, mut peer) = TestConnection::create_pair();
        let mut sz = Sz::new(1024);
        let mut state = TransferState::new("flow control".into());
        for byte in [0x11, 0x91, 0x13, 0x93] {
            peer.send(&[byte]).await.unwrap();
            if blocking {
                bounded(sz.read_next_header(&mut conn, &mut state)).await.unwrap();
            } else {
                bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap();
            }
            assert_eq!(sz.errors, 0);
            assert!(!state.is_finished);
            assert_idle(&mut peer).await;
        }
        peer.send(&wire(Header::from_number(ZFrameType::Challenge, 42))).await.unwrap();
        bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap();
        let ack = header(&mut peer).await;
        assert_eq!(ack.frame_type, ZFrameType::Ack);
        assert_eq!(ack.number(), 42);
    }
}

#[tokio::test]
async fn flow_control_and_stale_zrinit_preserve_the_data_ack_wait() {
    let file = file(b"abcd");
    let mut sender = Sender::with_streaming(&[file.path().to_path_buf()], true, false).await;
    sender.offer(0).await;
    sender.step().await;
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Data);
    sender.step().await;
    assert_eq!(sender.packet().await, (b"ab".to_vec(), true, true));

    for byte in [0x11, 0x91, 0x13, 0x93] {
        sender.peer.send(&[byte]).await.unwrap();
        sender.step().await;
        // No pending input: an ACK wait must remain a nonblocking idle update.
        sender.step().await;
        assert_idle(&mut sender.peer).await;
        assert_eq!(sender.sz.errors, 0);
        assert_eq!(sender.state.send_state.cur_bytes_transfered, 2);
    }
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert!(sender.state.send_state.finished_files.is_empty());
    sender.step().await;
    assert_idle(&mut sender.peer).await;
    sender.inject(Header::from_number(ZFrameType::Ack, 2)).await;
    sender.step().await;
    let data = header(&mut sender.peer).await;
    assert_eq!(data.frame_type, ZFrameType::Data);
    assert_eq!(data.number(), 2);
    sender.step().await;
    assert_eq!(sender.packet().await, (b"cd".to_vec(), true, false));
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Eof);
    sender.inject(Header::empty(ZFrameType::RIinit)).await;
    assert_eq!(sender.state.send_state.finished_files.len(), 1);
    sender.end_batch().await;
}

#[tokio::test]
async fn zfin_after_zeof_sends_oo_without_confirming_the_file() {
    let file = file(b"x");
    let mut sender = Sender::new(&[file.path().to_path_buf()], true).await;
    sender.offer(0).await;
    sender.step().await;
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Data);
    sender.step().await;
    assert_eq!(sender.packet().await, (b"x".to_vec(), true, false));
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Eof);
    sender.inject(Header::empty(ZFrameType::Fin)).await;
    assert!(sender.state.is_finished);
    assert!(sender.state.send_state.finished_files.is_empty());
    assert!(!sender.sz.transfered_file);
    let mut oo = [0; 2];
    bounded(sender.peer.read_exact(&mut oo)).await.unwrap();
    assert_eq!(&oo, b"OO");
}

fn corrupt_header(kind: HeaderType) -> Vec<u8> {
    let mut bytes = Header::empty(ZFrameType::Ack).build(kind, false);
    bytes[4] ^= 1; // Alter unescaped body data, not the framing or CRC encoding.
    bytes
}

#[tokio::test]
async fn malformed_headers_and_crc_errors_have_a_bounded_recovery_budget() {
    let malformed = [
        vec![b'?'],
        vec![ZPAD, b'?'],
        vec![ZPAD, ZDLE, b'?'],
        corrupt_header(HeaderType::Bin),
        corrupt_header(HeaderType::Bin32),
    ];
    for blocking in [false, true] {
        for bad in &malformed {
            let (mut conn, mut peer) = TestConnection::create_pair();
            let mut sz = Sz::new(1024);
            let mut state = TransferState::new("header errors".into());
            for attempt in 1..=4 {
                peer.send(bad).await.unwrap();
                let result = if blocking {
                    bounded(sz.read_next_header(&mut conn, &mut state)).await
                } else {
                    bounded(sz.update_transfer(&mut conn, &mut state)).await
                };
                assert_eq!(sz.errors, attempt);
                if attempt < 4 {
                    result.unwrap();
                    assert!(!state.is_finished);
                    assert_idle(&mut peer).await;
                } else {
                    assert!(result.is_err());
                    assert!(state.is_finished);
                    let mut abort = vec![0; ABORT_SEQ.len()];
                    bounded(peer.read_exact(&mut abort)).await.unwrap();
                    assert_eq!(abort, ABORT_SEQ);
                }
            }
        }
    }
}

#[tokio::test]
async fn a_valid_header_recovers_after_crc_error() {
    let (mut conn, mut peer) = TestConnection::create_pair();
    let mut sz = Sz::new(1024);
    let mut state = TransferState::new("CRC recovery".into());
    peer.send(&corrupt_header(HeaderType::Bin32)).await.unwrap();
    bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap();
    assert_eq!(sz.errors, 1);
    peer.send(&wire(Header::from_number(ZFrameType::Challenge, 17))).await.unwrap();
    bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap();
    assert_eq!(sz.errors, 0);
    assert_eq!(header(&mut peer).await.number(), 17);
    assert!(!state.is_finished);
}

#[tokio::test]
async fn cancel_requires_five_consecutive_can_bytes() {
    for blocking in [false, true] {
        let (mut conn, mut peer) = TestConnection::create_pair();
        let mut sz = Sz::new(1024);
        let mut state = TransferState::new("CAN sequence".into());
        let bytes = [ZDLE, ZDLE, ZDLE, b'?', ZDLE, ZDLE, 0x11, ZDLE, ZDLE, ZDLE, ZDLE, ZDLE];
        for (index, byte) in bytes.into_iter().enumerate() {
            peer.send(&[byte]).await.unwrap();
            if blocking {
                bounded(sz.read_next_header(&mut conn, &mut state)).await.unwrap();
            } else {
                bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap();
            }
            assert_eq!(state.is_finished, index == bytes.len() - 1);
            assert_eq!(sz.errors, usize::from(index >= 3), "CAN and flow control must not consume the error budget");
        }
        let mut abort = vec![0; ABORT_SEQ.len()];
        bounded(peer.read_exact(&mut abort)).await.unwrap();
        assert_eq!(abort, ABORT_SEQ);
    }
}

#[tokio::test]
async fn file_handshake_uses_the_same_flow_crc_and_cancel_policy() {
    let file = file(b"x");
    let paths = [file.path().to_path_buf()];
    let mut sender = Sender::new(&paths, true).await;
    let mut reply = vec![0x11, 0x91, 0x13];
    reply.extend(corrupt_header(HeaderType::Bin32));
    reply.extend(wire(Header::from_number(ZFrameType::RPos, 0)));
    sender.offer_result(&reply).await.unwrap();
    assert_eq!(sender.sz.errors, 0);
    sender.step().await;
    assert_eq!(header(&mut sender.peer).await.frame_type, ZFrameType::Data);

    for reply in [vec![ZDLE; 5], vec![0x11; 32], corrupt_header(HeaderType::Bin32).repeat(4)] {
        let mut sender = Sender::new(&paths, true).await;
        let result = sender.offer_result(&reply).await;
        if reply == vec![ZDLE; 5] {
            result.unwrap();
        } else {
            assert!(result.is_err());
        }
        assert!(sender.state.is_finished);
        assert!(sender.state.send_state.finished_files.is_empty());
        let mut abort = vec![0; ABORT_SEQ.len()];
        bounded(sender.peer.read_exact(&mut abort)).await.unwrap();
        assert_eq!(abort, ABORT_SEQ);
    }
}

struct FailedTransport {
    fail_try_read: bool,
}

#[async_trait]
impl Connection for FailedTransport {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    async fn read(&mut self, _: &mut [u8]) -> icy_net::Result<usize> {
        Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "test transport failure").into())
    }

    async fn try_read(&mut self, _: &mut [u8]) -> icy_net::Result<usize> {
        if self.fail_try_read {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "test transport failure").into())
        } else {
            Ok(0)
        }
    }

    async fn send(&mut self, _: &[u8]) -> icy_net::Result<()> {
        panic!("transport errors must propagate, not send protocol cancellation")
    }
}

#[tokio::test]
async fn transport_errors_and_partial_header_eof_are_fatal() {
    for fail_try_read in [false, true] {
        let mut conn = FailedTransport { fail_try_read };
        let mut sz = Sz::new(1024);
        let mut state = TransferState::new("transport failure".into());
        let error = bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap_err();
        assert_eq!(error.downcast_ref::<std::io::Error>().unwrap().kind(), std::io::ErrorKind::BrokenPipe);
        assert_eq!(sz.errors, 0);
    }

    let (mut conn, mut peer) = TestConnection::create_pair();
    peer.send(&[ZPAD]).await.unwrap();
    peer.shutdown_tx();
    let mut sz = Sz::new(1024);
    let mut state = TransferState::new("truncated header".into());
    let error = bounded(sz.update_transfer(&mut conn, &mut state)).await.unwrap_err();
    assert_eq!(error.to_string(), "Connection closed");
    assert_eq!(sz.errors, 0);
}

#[tokio::test]
async fn real_sender_and_receiver_complete_an_empty_file_batch() {
    let files = [file(b""), file(b"nonempty")];
    let paths: Vec<_> = files.iter().map(|file| file.path().to_path_buf()).collect();
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut sender = Zmodem::new(1024);
    let mut receiver = Zmodem::new(1024);
    let mut send_state = bounded(sender.initiate_send(&mut sender_conn, &paths)).await.unwrap();
    let mut recv_state = bounded(receiver.initiate_recv(&mut receiver_conn)).await.unwrap();

    bounded(async {
        tokio::join!(
            async {
                while !send_state.is_finished {
                    sender.update_transfer(&mut sender_conn, &mut send_state).await.unwrap();
                    tokio::task::yield_now().await;
                }
            },
            async {
                while !recv_state.is_finished {
                    receiver.update_transfer(&mut receiver_conn, &mut recv_state).await.unwrap();
                    tokio::task::yield_now().await;
                }
            }
        );
    })
    .await;
    assert_eq!(send_state.send_state.finished_files.len(), 2);
    assert_eq!(recv_state.recieve_state.finished_files.len(), 2);
    assert_eq!(send_state.send_state.total_bytes_transfered, 8);
    assert_eq!(recv_state.recieve_state.total_bytes_transfered, 8);
    for ((_, received), original) in recv_state.recieve_state.finished_files.iter().zip(files) {
        // Rz keeps its temporary files on completion; reclaim them for this test.
        let received = tempfile::TempPath::try_from_path(received).unwrap();
        assert_eq!(std::fs::read(&received).unwrap(), std::fs::read(original.path()).unwrap());
    }
}
