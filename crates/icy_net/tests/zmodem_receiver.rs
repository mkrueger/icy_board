use std::{future::Future, time::Duration};

use icy_net::{
    Connection,
    connection::channel::ChannelConnection,
    protocol::{Header, HeaderType, Protocol, TransferState, ZCRCE, ZCRCG, ZCRCW, ZFrameType, Zmodem},
};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("receiver regression timed out")
}

async fn receiver() -> (Zmodem, TransferState, ChannelConnection, ChannelConnection) {
    receiver_with_block_length(1024).await
}

async fn receiver_with_block_length(block_length: usize) -> (Zmodem, TransferState, ChannelConnection, ChannelConnection) {
    let (mut conn, mut peer) = ChannelConnection::create_pair();
    let mut protocol = Zmodem::new(block_length);
    let state = bounded(protocol.initiate_recv(&mut conn)).await.unwrap();
    assert_eq!(bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap().frame_type, ZFrameType::RIinit);
    (protocol, state, conn, peer)
}

#[tokio::test]
async fn receiver_accepts_large_streaming_subpackets_independently_of_upload_block_size() {
    for block_length in [1024, 8192] {
        for kind in [HeaderType::Bin, HeaderType::Bin32] {
            for size in [1024, 1025, 2048, 4096, 8192] {
                let (mut protocol, mut state, mut conn, mut peer) = receiver_with_block_length(block_length).await;
                let encode = |marker, data: &[u8]| {
                    if kind == HeaderType::Bin32 {
                        Zmodem::encode_subpacket_crc32(marker, data, false)
                    } else {
                        Zmodem::encode_subpacket_crc16(marker, data, false)
                    }
                };
                let data: Vec<u8> = (0..size).map(|index| (index % 256) as u8).collect();
                let mut expected = data.clone();
                expected.extend_from_slice(b"tail");
                let metadata = format!("large.bin\0{}\0", expected.len());
                let mut bytes = Header::empty(ZFrameType::File).build(kind, false);
                bytes.extend(encode(ZCRCW, metadata.as_bytes()));
                peer.send(&bytes).await.unwrap();
                bounded(protocol.update_transfer(&mut conn, &mut state)).await.unwrap();
                assert_eq!(bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap().frame_type, ZFrameType::RPos);

                let mut bytes = Header::from_number(ZFrameType::Data, 0).build(kind, false);
                bytes.extend(encode(ZCRCG, &data));
                bytes.extend(encode(ZCRCE, b"tail"));
                peer.send(&bytes).await.unwrap();
                bounded(async {
                    while state.recieve_state.cur_bytes_transfered < expected.len() as u64 {
                        protocol.update_transfer(&mut conn, &mut state).await.unwrap();
                    }
                })
                .await;
                assert_eq!(state.recieve_state.total_bytes_transfered, expected.len() as u64);
                assert_eq!(state.recieve_state.errors, 0);
                assert_eq!(state.recieve_state.warnings, 0);

                Header::from_number(ZFrameType::Eof, expected.len() as u32)
                    .write(&mut peer, kind, false)
                    .await
                    .unwrap();
                bounded(async {
                    while state.recieve_state.finished_files.is_empty() {
                        protocol.update_transfer(&mut conn, &mut state).await.unwrap();
                    }
                })
                .await;
                let path = &state.recieve_state.finished_files[0].1;
                let received = std::fs::read(path).unwrap();
                std::fs::remove_file(path).unwrap();
                assert_eq!(received, expected, "{size}-byte {kind:?} subpacket with {block_length}-byte uploads");
            }
        }
    }
}

#[tokio::test]
async fn receiver_keeps_a_bounded_limit_for_oversized_subpackets() {
    for block_length in [1024, 8192] {
        for kind in [HeaderType::Bin, HeaderType::Bin32] {
            let (mut protocol, mut state, mut conn, mut peer) = receiver_with_block_length(block_length).await;
            let encode = |marker, data: &[u8]| {
                if kind == HeaderType::Bin32 {
                    Zmodem::encode_subpacket_crc32(marker, data, false)
                } else {
                    Zmodem::encode_subpacket_crc16(marker, data, false)
                }
            };
            let mut bytes = Header::empty(ZFrameType::File).build(kind, false);
            bytes.extend(encode(ZCRCW, b"oversized.bin\x008193\x00"));
            peer.send(&bytes).await.unwrap();
            bounded(protocol.update_transfer(&mut conn, &mut state)).await.unwrap();
            bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap();
            let mut bytes = Header::from_number(ZFrameType::Data, 0).build(kind, false);
            bytes.extend(encode(ZCRCE, &vec![b'A'; 8193]));
            peer.send(&bytes).await.unwrap();
            let error = bounded(async {
                loop {
                    if let Err(error) = protocol.update_transfer(&mut conn, &mut state).await {
                        break error;
                    }
                }
            })
            .await;
            assert_eq!(error.to_string(), "subpacket overflow: length 8193 exceeds max 8192");
            assert_eq!(state.recieve_state.cur_bytes_transfered, 0, "oversized data is never written");
            assert!(state.recieve_state.finished_files.is_empty());
        }
    }
}

#[tokio::test]
async fn receiver_checks_crc_before_writing_large_subpackets() {
    for kind in [HeaderType::Bin, HeaderType::Bin32] {
        let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
        let encode = |marker, data: &[u8]| {
            if kind == HeaderType::Bin32 {
                Zmodem::encode_subpacket_crc32(marker, data, false)
            } else {
                Zmodem::encode_subpacket_crc16(marker, data, false)
            }
        };
        let data = vec![b'A'; 8192];
        let mut bytes = Header::empty(ZFrameType::File).build(kind, false);
        bytes.extend(encode(ZCRCW, b"crc.bin\x008192\x00"));
        peer.send(&bytes).await.unwrap();
        bounded(protocol.update_transfer(&mut conn, &mut state)).await.unwrap();
        bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap();

        let mut corrupt = encode(ZCRCE, &data);
        corrupt[0] = b'B';
        let mut bytes = Header::from_number(ZFrameType::Data, 0).build(kind, false);
        bytes.extend(corrupt);
        peer.send(&bytes).await.unwrap();
        bounded(async {
            while state.recieve_state.warnings == 0 {
                protocol.update_transfer(&mut conn, &mut state).await.unwrap();
            }
        })
        .await;
        assert_eq!(state.recieve_state.cur_bytes_transfered, 0);
        let retry = bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap();
        assert_eq!(retry.frame_type, ZFrameType::RPos);
        assert_eq!(retry.number(), 0);

        let mut bytes = Header::from_number(ZFrameType::Data, 0).build(kind, false);
        bytes.extend(encode(ZCRCE, &data));
        peer.send(&bytes).await.unwrap();
        bounded(async {
            while state.recieve_state.cur_bytes_transfered < data.len() as u64 {
                protocol.update_transfer(&mut conn, &mut state).await.unwrap();
            }
        })
        .await;
        Header::from_number(ZFrameType::Eof, data.len() as u32)
            .write(&mut peer, kind, false)
            .await
            .unwrap();
        bounded(async {
            while state.recieve_state.finished_files.is_empty() {
                protocol.update_transfer(&mut conn, &mut state).await.unwrap();
            }
        })
        .await;
        let path = &state.recieve_state.finished_files[0].1;
        let received = std::fs::read(path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(received, data, "only the CRC-validated retransmission is written");
    }
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

#[tokio::test]
async fn flow_control_bytes_before_a_header_are_not_errors() {
    let (mut protocol, mut state, mut conn, mut peer) = receiver().await;
    let mut bytes = Header::empty(ZFrameType::File).build(HeaderType::Bin32, false);
    bytes.extend(Zmodem::encode_subpacket_crc32(ZCRCW, b"name\x005\x00", false));
    // Senders like lrzsz follow a ZCRCW subpacket with XON; flow control may add XOFF with parity.
    bytes.extend_from_slice(&[0x11, 0x93]);
    peer.send(&bytes).await.unwrap();
    bounded(protocol.update_transfer(&mut conn, &mut state)).await.unwrap();
    let reply = bounded(Header::read(&mut peer, &mut 0)).await.unwrap().unwrap();
    assert_eq!(reply.frame_type, ZFrameType::RPos);

    let mut bytes = Header::from_number(ZFrameType::Data, 0).build(HeaderType::Bin32, false);
    bytes.extend(Zmodem::encode_subpacket_crc32(ZCRCE, b"hello", false));
    peer.send(&bytes).await.unwrap();
    bounded(async {
        while state.recieve_state.cur_bytes_transfered < 5 {
            protocol.update_transfer(&mut conn, &mut state).await.unwrap();
        }
    })
    .await;
    assert_eq!(state.recieve_state.errors, 0, "{:?}", state.recieve_state.output_log);
    assert_eq!(state.recieve_state.warnings, 0, "{:?}", state.recieve_state.output_log);
}
