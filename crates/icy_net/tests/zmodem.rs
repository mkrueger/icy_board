use icy_net::Connection;
use icy_net::crc::update_crc32;
use icy_net::protocol::zmodem::rz::read_subpacket;
use icy_net::protocol::{ZCRCE, Zmodem};

mod test_connection;
use test_connection::TestConnection;

// Simple CRC32 subpacket encoding test stays sync
#[test]
fn test_encode_subpckg_crc32() {
    let pck = Zmodem::encode_subpacket_crc32(ZCRCE, b"a\n", false);
    let expected = vec![0x61, 0x0a, 0x18, 0x68, 0xe5, 0x79, 0xd2, 0x0f];
    assert_eq!(expected, pck);
}

#[test]
fn test_crc32_vector() {
    let data = b"ABC";
    // Hand-computed using C logic
    let mut crc = 0xFFFF_FFFFu32;
    for b in data {
        crc = update_crc32(crc, *b);
    }
    crc = update_crc32(crc, ZCRCE);
    crc = !crc;
    assert_eq!(crc, 0xE9CF4C46);
}

#[tokio::test]
async fn test_subpckg_roundtrip_crc32() {
    let payload = b"foo_bar\n";
    let encoded = Zmodem::encode_subpacket_crc32(ZCRCE, payload, false);

    // Create a pair of connections and use one side for testing
    let (mut conn, mut _other) = TestConnection::create_pair();

    // Send the encoded data from the other side
    tokio::spawn(async move {
        _other.send(&encoded).await.unwrap();
    });

    // Decode (block_length > payload len)
    let (decoded, last, zack) = read_subpacket(&mut conn, 256, true, false).await.expect("decode subpacket");

    assert!(last, "Single subpacket should mark end of frame");
    assert!(!zack, "ZCRCE shouldn't request ACK");
    assert_eq!(decoded.as_slice(), payload, "Round-trip payload mismatch");
}

#[tokio::test]
async fn test_zmodem_simple_send() {
    use crate::test_connection::TestConnection;
    use icy_net::protocol::zmodem::rz::read_subpacket;
    use icy_net::protocol::{Header, HeaderType, Protocol, ZFrameType, Zmodem, str_from_null_terminated_utf8_unchecked};
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Prepare a temp file
    let mut f = NamedTempFile::new().unwrap();
    let data = vec![1u8, 2, 5, 10, 11, 12, 13, 14, 15, 16];
    f.write_all(&data).unwrap();
    let path = f.path().to_path_buf();

    // Create paired connections: sender (a) and receiver simulation (b)
    let (mut a, mut b) = TestConnection::create_pair();

    // Instantiate protocol sender
    let mut z = Zmodem::new(4);

    // Spawn sender task
    let sender_handle = tokio::spawn(async move {
        let mut state = z.initiate_send(&mut a, std::slice::from_ref(&path)).await.expect("init send failed");

        while !state.is_finished {
            z.update_transfer(&mut a, &mut state).await.expect("update failed");
            // Small yield to allow receiver to process
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        (z, a, state)
    });

    // Receiver simulation
    let mut can_count = 0usize;
    let mut saw_file = false;
    let mut saw_data = false;
    let mut saw_eof = false;
    let mut saw_fin = false;
    let mut injected_handshake = false;
    let mut received_data = Vec::new();

    // Give sender time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Receiver loop with timeout
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let maybe_header = Header::read(&mut b, &mut can_count).await;
            let header = match maybe_header {
                Ok(Some(h)) => h,
                Ok(None) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
            };

            match header.frame_type {
                ZFrameType::RQInit => {
                    if !injected_handshake {
                        // Advertise CANFC32 (CRC32) and ESCCTL so sender uses 32-bit CRC & escapes
                        let caps = icy_net::protocol::zmodem::constants::zrinit_flag::CANFC32 | icy_net::protocol::zmodem::constants::zrinit_flag::ESCCTL;
                        // f0 = caps; p0/p1 (block size) left 0 => streaming/nonstop mode
                        Header::from_flags(ZFrameType::RIinit, 0, 0, 0, caps)
                            .write(&mut b, HeaderType::Hex, true) // pass escape_ctrl_chars = true to match ESCCTL
                            .await
                            .expect("write ZRINIT with capabilities");
                        injected_handshake = true;
                    }
                }
                ZFrameType::SInit => {
                    assert_eq!(
                        header.f0(),
                        icy_net::protocol::zmodem::constants::zsinit_flag::TESCCTL,
                        "ZSINIT escape request belongs in ZF0"
                    );
                    // Read the attention string subpacket (usually empty or minimal)
                    // Data following a HEX header is always protected by CRC-16.
                    let (_attn_data, last, _) = read_subpacket(&mut b, 1024, false, true).await.expect("read ZSINIT subpacket");
                    assert!(last, "ZSINIT subpacket should be last");

                    // Send ZACK to acknowledge ZSINIT
                    Header::from_number(ZFrameType::Ack, 0)
                        .write(&mut b, HeaderType::Hex, false)
                        .await
                        .expect("write ZACK after ZSINIT");
                }
                ZFrameType::File => {
                    // Read the file info subpacket
                    let (block, last, _) = read_subpacket(&mut b, 1024, true, false).await.expect("read file subpacket");
                    assert!(last, "File header subpacket should be last in frame");
                    let name = str_from_null_terminated_utf8_unchecked(&block);
                    let expected_name = f.path().file_name().unwrap().to_string_lossy();
                    assert!(name.contains(expected_name.as_ref()));
                    saw_file = true;

                    // Send ZRPOS (offset 0). (Alternative: send Ack.)
                    Header::from_number(ZFrameType::RPos, 0)
                        .write(&mut b, HeaderType::Hex, false)
                        .await
                        .expect("write ZRPOS");
                }
                ZFrameType::Data => {
                    // Read the data subpacket
                    let (payload, last, zack) = read_subpacket(&mut b, 1024, true, false).await.expect("read data subpacket");
                    assert!(last, "ZCRCW/ZCRCE should end each segmented data frame");
                    received_data.extend_from_slice(&payload);
                    saw_data = true;

                    // Only send ACK if the subpacket explicitly requested it (ZCRCQ or ZCRCW).
                    if zack {
                        Header::from_number(ZFrameType::Ack, received_data.len() as u32)
                            .write(&mut b, HeaderType::Hex, false)
                            .await
                            .expect("write ACK after data");
                    }
                }
                ZFrameType::Eof => {
                    saw_eof = true;
                    // Send ZRINIT to acknowledge EOF
                    Header::empty(ZFrameType::RIinit)
                        .write(&mut b, HeaderType::Hex, false)
                        .await
                        .expect("write ZRINIT after EOF");
                }
                ZFrameType::Fin => {
                    saw_fin = true;
                    // Echo ZFIN to complete session
                    Header::empty(ZFrameType::Fin).write(&mut b, HeaderType::Hex, false).await.expect("echo ZFIN");

                    // Send OO sequence
                    b.send(b"OO").await.expect("send OO");
                    break;
                }
                other => {
                    log::error!("Unexpected header type: {:?}", other);
                }
            }
        }
    })
    .await;

    // Check timeout didn't occur
    timeout.expect("Test timed out");

    // Wait for sender to complete
    let (_z, _a, state) = sender_handle.await.expect("Sender task failed");

    assert!(state.is_finished, "Transfer should be finished");
    assert!(saw_file, "File header not observed");
    assert!(saw_data, "Data frame not observed");
    assert_eq!(received_data, data, "Segmented transfer payload mismatch");
    assert!(saw_eof, "EOF frame not observed");
    assert!(saw_fin, "FIN frame not observed");
}

#[tokio::test]
async fn test_encode_char_table() {
    // Test each byte value individually to avoid packet concatenation issues
    for i in 0..=255u8 {
        // Create a fresh pair for each test to avoid contamination
        let (mut conn, mut feeder) = TestConnection::create_pair();

        let data = vec![i];
        let encoded = Zmodem::encode_subpacket_crc32(0x6B, &data, true);

        // Send this single encoded packet
        tokio::spawn(async move {
            feeder.send(&encoded).await.unwrap();
        });

        // Give time for the data to be sent
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        // Read and verify this single encoded byte
        let (decoded, last, _) = read_subpacket(&mut conn, 1024, true, true)
            .await
            .unwrap_or_else(|_| panic!("decode subpacket for byte {}", i));
        assert!(last, "Each generated subpacket should terminate");
        assert_eq!(data, decoded, "Mismatch at byte {i}");
    }
}

// Regression test: ensure reading a recorded subpacket does not panic
#[tokio::test]
async fn subpacket_bug() {
    let bytes = include_bytes!("sub_package_test1.dat").to_vec();
    let (mut conn, mut feeder) = TestConnection::create_pair();

    // Feed the test data from the other end
    tokio::spawn(async move {
        feeder.send(&bytes).await.unwrap();
    });

    // Just attempt to parse; expect Ok
    let _ = read_subpacket(&mut conn, 1024, true, true).await.expect("should parse recorded subpacket");
}

#[tokio::test]
async fn header_bin32_escctl_crc32_roundtrip() {
    use icy_net::protocol::{Header, HeaderType, ZFrameType};
    use test_connection::TestConnection;

    let (mut a, mut b) = TestConnection::create_pair();
    // Data chosen to force escapes: includes 0x00, 0x11 (XON), 0x13 (XOFF)
    let hdr = Header::from_flags(ZFrameType::File, 0x00, 0x11, 0x13, 0x40);
    tokio::spawn(async move {
        hdr.write(&mut a, HeaderType::Bin32, true).await.unwrap();
    });
    let mut can_count = 0;
    let read = Header::read(&mut b, &mut can_count).await.unwrap().unwrap();
    assert_eq!(read.frame_type, ZFrameType::File);
    assert_eq!(read.f3(), 0x00);
    assert_eq!(read.f2(), 0x11);
    assert_eq!(read.f1(), 0x13);
    assert_eq!(read.f0(), 0x40);
}

#[test]
fn binary_header_escapes_frame_type_with_escctl() {
    use icy_net::protocol::{Header, HeaderType, ZBIN32, ZDLE, ZFrameType, ZPAD};

    let built = Header::empty(ZFrameType::Data).build(HeaderType::Bin32, true);
    assert_eq!(&built[..5], &[ZPAD, ZDLE, ZBIN32, ZDLE, (ZFrameType::Data as u8) ^ 0x40]);
}

#[test]
fn escctl_protects_cr_after_parity_at_sign() {
    use icy_net::protocol::ZDLE;
    use icy_net::protocol::zmodem::append_zdle_encoded;

    let mut encoded = Vec::new();
    append_zdle_encoded(&mut encoded, &[0xC0, b'\r'], true);
    assert_eq!(encoded, vec![0xC0, ZDLE, b'\r' ^ 0x40]);
}

#[tokio::test]
async fn malformed_or_oversized_subpackets_are_rejected() {
    use icy_net::protocol::zmodem::read_zdle_bytes;
    use icy_net::protocol::{ZCRCE, ZDLE};

    let (mut invalid_conn, mut invalid_feeder) = TestConnection::create_pair();
    invalid_feeder.send(&[ZDLE, b'z']).await.unwrap();
    assert!(read_subpacket(&mut invalid_conn, 16, true, false).await.is_err());

    let (mut oversized_conn, mut oversized_feeder) = TestConnection::create_pair();
    oversized_feeder.send(b"abcd").await.unwrap();
    assert!(read_subpacket(&mut oversized_conn, 3, true, false).await.is_err());

    let (mut header_conn, mut header_feeder) = TestConnection::create_pair();
    header_feeder.send(&[ZDLE, ZCRCE]).await.unwrap();
    assert!(read_zdle_bytes(&mut header_conn, 1).await.is_err());
}

#[tokio::test]
async fn sender_preserves_nonzero_zrpos_resume_offset() {
    use icy_net::protocol::{Header, HeaderType, Protocol, ZFrameType};
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut file = NamedTempFile::new().unwrap();
    let data = b"resume payload".to_vec();
    file.write_all(&data).unwrap();
    let path = file.path().to_path_buf();
    let resume_offset = 7u32;

    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut protocol = Zmodem::new(1024);
    let sender = tokio::spawn(async move {
        let mut state = protocol.initiate_send(&mut sender_conn, &[path]).await.unwrap();
        while !state.is_finished {
            protocol.update_transfer(&mut sender_conn, &mut state).await.unwrap();
            tokio::task::yield_now().await;
        }
    });

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut can_count = 0;
        loop {
            let header = Header::read(&mut receiver_conn, &mut can_count).await.unwrap().unwrap();
            match header.frame_type {
                ZFrameType::RQInit => {
                    let caps = icy_net::protocol::zmodem::constants::zrinit_flag::CANFC32 | icy_net::protocol::zmodem::constants::zrinit_flag::CANOVIO;
                    Header::from_flags(ZFrameType::RIinit, 0, 0, 0, caps)
                        .write(&mut receiver_conn, HeaderType::Hex, false)
                        .await
                        .unwrap();
                }
                ZFrameType::File => {
                    read_subpacket(&mut receiver_conn, 1024, true, false).await.unwrap();
                    Header::from_number(ZFrameType::RPos, resume_offset)
                        .write(&mut receiver_conn, HeaderType::Hex, false)
                        .await
                        .unwrap();
                }
                ZFrameType::Data => {
                    assert_eq!(header.number(), resume_offset);
                    let (payload, last, _) = read_subpacket(&mut receiver_conn, 1024, true, false).await.unwrap();
                    assert!(last);
                    assert_eq!(payload, data[resume_offset as usize..]);
                }
                ZFrameType::Eof => {
                    assert_eq!(header.number(), data.len() as u32);
                    Header::empty(ZFrameType::RIinit)
                        .write(&mut receiver_conn, HeaderType::Hex, false)
                        .await
                        .unwrap();
                }
                ZFrameType::Fin => {
                    Header::empty(ZFrameType::Fin).write(&mut receiver_conn, HeaderType::Hex, false).await.unwrap();
                    break;
                }
                other => panic!("unexpected frame during resume test: {other:?}"),
            }
        }
    })
    .await
    .expect("resume transfer timed out");

    sender.await.unwrap();
}

/// Regression test: Verify the complete ZRQINIT -> ZRINIT handshake works.
/// This was a bug where the receiver would log "will send ZRINIT" but never actually send it,
/// causing the transfer to hang in an infinite loop.
#[tokio::test]
async fn test_zrqinit_zrinit_handshake() {
    use icy_net::protocol::{Header, HeaderType, Protocol, ZFrameType, Zmodem};
    use test_connection::TestConnection;

    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Create receiver protocol
    let mut zmodem = Zmodem::new(1024);

    // Initiate receive - this sends the initial ZRINIT
    let mut state = zmodem.initiate_recv(&mut receiver_conn).await.expect("initiate_recv failed");

    // Read the initial ZRINIT from receiver
    let mut can_count = 0;
    let initial_zrinit = Header::read(&mut sender_conn, &mut can_count).await.unwrap().unwrap();
    assert_eq!(initial_zrinit.frame_type, ZFrameType::RIinit, "Expected initial ZRINIT from receiver");

    // Spawn a task to send ZRQINIT and then read the response
    let sender_handle = tokio::spawn(async move {
        // Sender sends ZRQINIT
        let zrqinit = Header::empty(ZFrameType::RQInit);
        zrqinit.write(&mut sender_conn, HeaderType::Hex, false).await.unwrap();

        // Wait for and read ZRINIT response
        let mut can_count = 0;

        tokio::time::timeout(std::time::Duration::from_secs(2), Header::read(&mut sender_conn, &mut can_count))
            .await
            .expect("Timeout waiting for ZRINIT response - receiver didn't send ZRINIT!")
            .expect("Failed to read header")
            .expect("No header received")
    });

    // Give sender time to send ZRQINIT
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Receiver processes the ZRQINIT - this MUST trigger sending ZRINIT
    zmodem.update_transfer(&mut receiver_conn, &mut state).await.expect("update_transfer failed");

    // Verify sender received ZRINIT
    let response = sender_handle.await.expect("Sender task failed");
    assert_eq!(
        response.frame_type,
        ZFrameType::RIinit,
        "Expected ZRINIT response after ZRQINIT, got {:?}",
        response.frame_type
    );
}

/// Test that Hex headers are built with CR LF line ending as per ZModem spec.
/// This is important for compatibility with older BBS systems.
#[test]
fn test_hex_header_ends_with_crlf() {
    use icy_net::protocol::{Header, HeaderType, ZFrameType};

    // Test ZRINIT header (should have XON after CR LF)
    let zrinit = Header::empty(ZFrameType::RIinit);
    let built = zrinit.build(HeaderType::Hex, false);

    // Find CR LF sequence - should be near the end
    let crlf_pos = built.windows(2).position(|w| w == b"\r\n");
    assert!(crlf_pos.is_some(), "Hex header should contain CR LF sequence");

    let pos = crlf_pos.unwrap();
    // After CR LF, there should be XON (0x11) for non-ACK/FIN frames
    assert!(pos + 2 < built.len() && built[pos + 2] == 0x11, "ZRINIT header should have XON after CR LF");

    // Test ZACK header (should NOT have XON after CR LF)
    let zack = Header::empty(ZFrameType::Ack);
    let built_ack = zack.build(HeaderType::Hex, false);

    let crlf_pos_ack = built_ack.windows(2).position(|w| w == b"\r\n");
    assert!(crlf_pos_ack.is_some(), "ZACK Hex header should contain CR LF sequence");

    let pos_ack = crlf_pos_ack.unwrap();
    // ZACK should end with CR LF (no XON)
    assert_eq!(pos_ack + 2, built_ack.len(), "ZACK header should end with CR LF without XON");

    // Test ZFIN header (should NOT have XON after CR LF)
    let zfin = Header::empty(ZFrameType::Fin);
    let built_fin = zfin.build(HeaderType::Hex, false);

    let crlf_pos_fin = built_fin.windows(2).position(|w| w == b"\r\n");
    assert!(crlf_pos_fin.is_some(), "ZFIN Hex header should contain CR LF sequence");

    let pos_fin = crlf_pos_fin.unwrap();
    // ZFIN should end with CR LF (no XON)
    assert_eq!(pos_fin + 2, built_fin.len(), "ZFIN header should end with CR LF without XON");
}
