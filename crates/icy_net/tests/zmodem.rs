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
                    println!("No header available, continuing...");
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => {
                    println!("Header read error: {:?}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
            };

            println!("Received header: {:?}", header.frame_type);

            match header.frame_type {
                ZFrameType::RQInit => {
                    println!("Got ZRQINIT, sending ZRINIT");
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
                ZFrameType::File => {
                    println!("Got ZFILE, reading subpacket and sending ZRPOS");
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
                    println!("Got ZDATA, reading subpacket");
                    // Read the data subpacket
                    let (payload, last, zack) = read_subpacket(&mut b, 1024, true, false).await.expect("read data subpacket");
                    println!("Data subpacket: last={}, zack={}, len={}", last, zack, payload.len());
                    assert!(last, "Expect single subpacket for tiny test data");
                    assert_eq!(payload, data);
                    saw_data = true;

                    // Only send ACK if the subpacket explicitly requested it (ZCRCQ or ZCRCW).
                    if zack {
                        println!("Sending ACK as requested");
                        Header::empty(ZFrameType::Ack)
                            .write(&mut b, HeaderType::Hex, false)
                            .await
                            .expect("write ACK after data");
                    }
                }
                ZFrameType::Eof => {
                    println!("Got ZEOF, sending ZRINIT");
                    saw_eof = true;
                    // Send ZRINIT to acknowledge EOF
                    Header::empty(ZFrameType::RIinit)
                        .write(&mut b, HeaderType::Hex, false)
                        .await
                        .expect("write ZRINIT after EOF");
                }
                ZFrameType::Fin => {
                    println!("Got ZFIN, echoing and sending OO");
                    saw_fin = true;
                    // Echo ZFIN to complete session
                    Header::empty(ZFrameType::Fin).write(&mut b, HeaderType::Hex, false).await.expect("echo ZFIN");

                    // Send OO sequence
                    b.send(b"OO").await.expect("send OO");
                    break;
                }
                other => {
                    println!("Unexpected header type: {:?}", other);
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
