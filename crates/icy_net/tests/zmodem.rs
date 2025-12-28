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
    let data = vec![1u8, 2, 5, 10];
    f.write_all(&data).unwrap();
    let path = f.path().to_path_buf();

    // Create paired connections: sender (a) and receiver simulation (b)
    let (mut a, mut b) = TestConnection::create_pair();

    // Instantiate protocol sender
    let mut z = Zmodem::new(1024);

    // Spawn sender task
    let sender_handle = tokio::spawn(async move {
        let mut state = z.initiate_send(&mut a, &[path.clone()]).await.expect("init send failed");

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
                    // Read the attention string subpacket (usually empty or minimal)
                    let (_attn_data, last, _) = read_subpacket(&mut b, 1024, true, false).await.expect("read ZSINIT subpacket");
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
                    assert!(last, "Expect single subpacket for tiny test data");
                    assert_eq!(payload, data);
                    saw_data = true;

                    // Only send ACK if the subpacket explicitly requested it (ZCRCQ or ZCRCW).
                    if zack {
                        Header::empty(ZFrameType::Ack)
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
            .expect(&format!("decode subpacket for byte {}", i));
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
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            Header::read(&mut sender_conn, &mut can_count),
        )
        .await
        .expect("Timeout waiting for ZRINIT response - receiver didn't send ZRINIT!")
        .expect("Failed to read header")
        .expect("No header received");

        response
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
    assert!(
        pos + 2 < built.len() && built[pos + 2] == 0x11,
        "ZRINIT header should have XON after CR LF"
    );

    // Test ZACK header (should NOT have XON after CR LF)
    let zack = Header::empty(ZFrameType::Ack);
    let built_ack = zack.build(HeaderType::Hex, false);

    let crlf_pos_ack = built_ack.windows(2).position(|w| w == b"\r\n");
    assert!(crlf_pos_ack.is_some(), "ZACK Hex header should contain CR LF sequence");

    let pos_ack = crlf_pos_ack.unwrap();
    // ZACK should end with CR LF (no XON)
    assert_eq!(
        pos_ack + 2,
        built_ack.len(),
        "ZACK header should end with CR LF without XON"
    );

    // Test ZFIN header (should NOT have XON after CR LF)
    let zfin = Header::empty(ZFrameType::Fin);
    let built_fin = zfin.build(HeaderType::Hex, false);

    let crlf_pos_fin = built_fin.windows(2).position(|w| w == b"\r\n");
    assert!(crlf_pos_fin.is_some(), "ZFIN Hex header should contain CR LF sequence");

    let pos_fin = crlf_pos_fin.unwrap();
    // ZFIN should end with CR LF (no XON)
    assert_eq!(
        pos_fin + 2,
        built_fin.len(),
        "ZFIN header should end with CR LF without XON"
    );
}
