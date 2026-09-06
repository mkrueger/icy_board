use std::{future::Future, time::Duration};

use icy_net::{
    Connection,
    connection::channel::ChannelConnection,
    protocol::{Header, HeaderType, Protocol, TransferState, ZCRCW, ZFrameType, Zmodem},
};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("receiver regression timed out")
}

async fn receiver() -> (Zmodem, TransferState, ChannelConnection, ChannelConnection) {
    let (mut conn, mut peer) = ChannelConnection::create_pair();
    let mut protocol = Zmodem::new(1024);
    let state = bounded(protocol.initiate_recv(&mut conn)).await.unwrap();
    assert_eq!(bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap().frame_type, ZFrameType::RIinit);
    (protocol, state, conn, peer)
}

async fn metadata(info: &[u8], kind: HeaderType) -> (icy_net::Result<()>, TransferState, ChannelConnection) {
    let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
    let mut bytes = Header::empty(ZFrameType::File).build(kind, false);
    bytes.extend(if kind == HeaderType::Bin32 {
        Zmodem::encode_subpacket_crc32(ZCRCW, info, false)
    } else {
        Zmodem::encode_subpacket_crc16(ZCRCW, info, false)
    });
    peer.send(&bytes).await.unwrap();
    let result = bounded(protocol.update_transfer(&mut conn, &mut state)).await;
    (result, state, peer)
}

#[tokio::test]
async fn malformed_metadata_is_rejected_without_panicking_or_accepting_a_file() {
    for kind in [HeaderType::Bin, HeaderType::Bin32, HeaderType::Hex] {
        for info in [
            b"".as_slice(),
            b"filename",
            b"\0",
            b"name\018446744073709551616\0",
            b"name\0999999999999999999999999999999999999999999999999999\0",
            b"name\0-1\0",
            b"name\012x\0",
        ] {
            let (result, state, mut peer) = metadata(info, kind).await;
            assert!(result.is_err(), "accepted {info:?}");
            assert!(state.recieve_state.finished_files.is_empty());
            assert_eq!(state.recieve_state.cur_bytes_transfered, 0);
            let mut abort = [0; 1];
            assert_eq!(peer.try_read(&mut abort).await.unwrap(), 1);
            assert_eq!(abort[0], 0x18, "invalid metadata must cancel, not send ZRPOS");
        }
    }
}

#[tokio::test]
async fn valid_metadata_uses_wire_offsets_for_legacy_and_utf8_names() {
    for (info, name, size) in [
        (b"\xff\0".as_slice(), "ÿ", 0),
        (b"\xff\0123\0", "ÿ", 123),
        ("ä.txt\0".as_bytes(), "ä.txt", 0),
        ("ä.txt\0123 777 644 0\0".as_bytes(), "ä.txt", 123),
        (b"name\0", "name", 0),
        (b"name\0\0", "name", 0),
        (b"name\00042\0", "name", 42),
        (b"name\018446744073709551615\0", "name", u64::MAX),
    ] {
        for kind in [HeaderType::Bin, HeaderType::Bin32, HeaderType::Hex] {
            let (result, state, mut peer) = metadata(info, kind).await;
            result.unwrap();
            assert_eq!(state.recieve_state.file_name, name);
            assert_eq!(state.recieve_state.file_size, size);
            assert!(state.recieve_state.finished_files.is_empty());
            let reply = bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap();
            assert_eq!(reply.frame_type, ZFrameType::RPos);
            assert_eq!(reply.number(), 0);
        }
    }
}

#[tokio::test]
async fn short_metadata_blocks_never_panic() {
    // Exhaust all one-byte names (including no terminator) and optional size.
    for byte in 0..=255u8 {
        let (result, _, _) = metadata(&[byte], HeaderType::Bin32).await;
        assert!(result.is_err());
        let (result, _, _) = metadata(&[byte, 0], HeaderType::Bin32).await;
        assert_eq!(result.is_ok(), byte != 0);
    }
}

#[tokio::test]
async fn receiver_consumes_oo_but_not_following_board_input() {
    let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
    let mut bytes = Header::empty(ZFrameType::Fin).build(HeaderType::Hex, false);
    bytes.extend_from_slice(b"OONEXT\r");
    peer.send(&bytes).await.unwrap();
    bounded(protocol.update_transfer(&mut conn, &mut state)).await.unwrap();
    assert!(state.is_finished);
    let mut remaining = [0; 5];
    bounded(conn.read_exact(&mut remaining)).await.unwrap();
    assert_eq!(&remaining, b"NEXT\r");
}

#[tokio::test]
async fn receiver_waits_for_fragmented_oo_after_its_zfin_reply() {
    let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
    Header::empty(ZFrameType::Fin).write(&mut peer, HeaderType::Hex, false).await.unwrap();
    bounded(async {
        let (result, ()) = tokio::join!(protocol.update_transfer(&mut conn, &mut state), async {
            assert_eq!(Header::read(&mut peer, &mut 0).await.unwrap().unwrap().frame_type, ZFrameType::Fin);
            tokio::time::sleep(Duration::from_millis(10)).await;
            peer.send(b"O").await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
            peer.send(b"ONEXT").await.unwrap();
        });
        result.unwrap();
    })
    .await;
    assert!(state.is_finished);
    let mut remaining = [0; 4];
    bounded(conn.read_exact(&mut remaining)).await.unwrap();
    assert_eq!(&remaining, b"NEXT");
}

#[tokio::test]
async fn missing_or_partial_oo_does_not_hang_cleanup() {
    for trailer in [b"".as_slice(), b"O"] {
        let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
        let mut bytes = Header::empty(ZFrameType::Fin).build(HeaderType::Hex, false);
        bytes.extend_from_slice(trailer);
        peer.send(&bytes).await.unwrap();
        bounded(protocol.update_transfer(&mut conn, &mut state)).await.unwrap();
        assert!(state.is_finished);
    }
}

#[tokio::test]
async fn disconnect_during_cleanup_finishes_but_header_eof_is_an_error() {
    for finishing in [false, true] {
        let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
        if finishing {
            Header::empty(ZFrameType::Fin).write(&mut peer, HeaderType::Hex, false).await.unwrap();
        }
        peer.shutdown().await.unwrap();
        let result = bounded(protocol.update_transfer(&mut conn, &mut state)).await;
        if finishing {
            result.unwrap();
            assert!(state.is_finished);
        } else {
            assert!(result.is_err(), "EOF must propagate, not repeat header reads forever");
        }
    }
}
