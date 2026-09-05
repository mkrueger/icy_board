use std::{fs, io::Write, path::PathBuf};

use icy_net::{
    Connection,
    protocol::{Protocol, XYModemVariant, XYmodem},
};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

mod test_connection;
use test_connection::{TestConnection, test_receiver, test_sender};

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;

// CRC16 calculation helper (XMODEM CRC-16)
fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[tokio::test]
async fn test_send_ymodem() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let data = vec![1u8, 2, 5, 10];

    let file_name = "foo.bar";
    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();

    // Rename the temp file to match the expected name
    let temp_path = named_temp.path().parent().unwrap().join(file_name);
    fs::rename(named_temp.path(), &temp_path).unwrap();

    // Expected data that sender should send
    let mut expected = vec![SOH, 0x00, 0xFF];
    let mut header_data = Vec::new();
    header_data.extend_from_slice(file_name.as_bytes());
    header_data.push(0); // null terminator
    header_data.extend_from_slice(b"4"); // file size as string
    header_data.resize(128, 0);

    expected.extend_from_slice(&header_data);
    let header_crc = crc16(&header_data);
    expected.push((header_crc >> 8) as u8);
    expected.push((header_crc & 0xFF) as u8);

    let mut padded_data = data.clone();
    padded_data.resize(128, 0x1A);
    expected.extend_from_slice(&[SOH, 0x01, 0xFE]);
    expected.extend_from_slice(&padded_data);
    let data_crc = crc16(&padded_data);
    expected.push((data_crc >> 8) as u8);
    expected.push((data_crc & 0xFF) as u8);
    expected.push(EOT);

    // Spawn receiver simulation
    let expected_clone = expected.clone();
    tokio::spawn(async move {
        let mut received = Vec::new();

        // Send initial 'C' to request CRC mode
        receiver_conn.send(b"C").await.unwrap();

        // Read header block
        let mut buf = vec![0u8; 133];
        receiver_conn.read(&mut buf).await.unwrap();
        received.extend_from_slice(&buf);
        receiver_conn.send(&[ACK]).await.unwrap();

        // Send 'C' for data transfer
        receiver_conn.send(b"C").await.unwrap();

        // Read data block
        receiver_conn.read(&mut buf).await.unwrap();
        received.extend_from_slice(&buf);
        receiver_conn.send(&[ACK]).await.unwrap();

        // Read EOT
        let mut eot_buf = [0u8; 1];
        receiver_conn.read(&mut eot_buf).await.unwrap();
        assert_eq!(eot_buf[0], EOT);
        received.push(EOT);
        receiver_conn.send(&[NAK]).await.unwrap();
        receiver_conn.read(&mut eot_buf).await.unwrap();
        assert_eq!(eot_buf[0], EOT);
        receiver_conn.send(&[ACK]).await.unwrap();

        // Send 'C' for end of batch
        receiver_conn.send(b"C").await.unwrap();

        // Read end-of-batch block (empty header)
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Verify the first part (actual file transfer) matches expected
        assert_eq!(received[..expected_clone.len()], expected_clone[..]);
    });

    let state = test_sender(&mut sender_conn, &mut protocol, std::slice::from_ref(&temp_path)).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);

    // Clean up
    fs::remove_file(&temp_path).ok();
}

#[tokio::test]
async fn test_recv_ymodem() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let orig_data = vec![1u8, 2, 5, 10];
    let file_name = "foo.bar";

    // Build the YModem packet
    let mut packet = vec![SOH, 0x00, 0xFF];
    let mut header_data = Vec::new();
    header_data.extend_from_slice(file_name.as_bytes());
    header_data.push(0); // null terminator
    header_data.extend_from_slice(b"4"); // file size as string
    header_data.resize(128, 0);

    packet.extend_from_slice(&header_data);
    let header_crc = crc16(&header_data);
    packet.push((header_crc >> 8) as u8);
    packet.push((header_crc & 0xFF) as u8);

    let mut data = orig_data.clone();
    let data_packet = vec![SOH, 0x01, 0xFE];
    packet.extend_from_slice(&data_packet);
    data.resize(128, 0x1A);
    packet.extend_from_slice(&data);
    let data_crc = crc16(&data);
    packet.push((data_crc >> 8) as u8);
    packet.push((data_crc & 0xFF) as u8);

    // No next file - empty header block
    let empty_packet = vec![SOH, 0x00, 0xFF];
    let empty_block = vec![0u8; 128];
    let empty_crc = crc16(&empty_block);

    // Spawn sender simulation
    tokio::spawn(async move {
        // Wait for initial 'C'
        let mut buf = [0u8; 1];
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], b'C');

        // Send header block
        sender_conn.send(&packet[0..133]).await.unwrap();

        // Wait for ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);

        // Wait for 'C' for data transfer
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], b'C');

        // Send data block
        sender_conn.send(&packet[133..266]).await.unwrap();

        // Wait for ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);

        // Send first EOT
        sender_conn.send(&[EOT]).await.unwrap();

        // Wait for NAK (first EOT gets NAK)
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], NAK);

        // Send second EOT
        sender_conn.send(&[EOT]).await.unwrap();

        // Wait for ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);

        // Wait for 'C' for next file
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], b'C');

        // Send empty header block (end of batch)
        let mut end_packet = empty_packet.clone();
        end_packet.extend_from_slice(&empty_block);
        end_packet.push((empty_crc >> 8) as u8);
        end_packet.push((empty_crc & 0xFF) as u8);
        sender_conn.send(&end_packet).await.unwrap();

        // Wait for final ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);
    });

    let state = test_receiver(&mut receiver_conn, &mut protocol).await;

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    // YModem knows the actual file size, so it should only have the original 4 bytes
    assert_eq!(loaded_data, orig_data);
}

#[tokio::test]
async fn test_ymodem_multiple_files() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut protocol = XYmodem::new(XYModemVariant::YModem);

    // Create two test files
    let data1 = vec![1u8, 2, 3, 4, 5];
    let data2: Vec<u8> = vec![10u8, 20, 30];

    let mut temp1 = NamedTempFile::new().unwrap();
    temp1.as_file_mut().write_all(&data1).unwrap();

    let mut temp2 = NamedTempFile::new().unwrap();
    temp2.as_file_mut().write_all(&data2).unwrap();

    // Spawn receiver simulation
    tokio::spawn(async move {
        // First file header
        receiver_conn.send(b"C").await.unwrap();
        let mut buf = vec![0u8; 133];
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // First file data
        receiver_conn.send(b"C").await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // First file EOT
        let mut eot_buf = [0u8; 1];
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[NAK]).await.unwrap();
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Second file header
        receiver_conn.send(b"C").await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Second file data
        receiver_conn.send(b"C").await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Second file EOT
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[NAK]).await.unwrap();
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // End of batch
        receiver_conn.send(b"C").await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();
    });

    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(temp1.path()), PathBuf::from(temp2.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 2);
    assert_eq!(state.send_state.total_bytes_transfered, (data1.len() + data2.len()) as u64);
}

#[tokio::test]
async fn test_ymodem_large_file() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut protocol = XYmodem::new(XYModemVariant::YModem);

    // Create a file larger than one block
    let data: Vec<u8> = (0..300).map(|i| (i % 256) as u8).collect();

    let mut temp = NamedTempFile::new().unwrap();
    temp.as_file_mut().write_all(&data).unwrap();

    // Spawn receiver simulation
    tokio::spawn(async move {
        let mut buf = vec![0u8; 133];
        let mut eot_buf = [0u8; 1];

        // File header
        receiver_conn.send(b"C").await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Request data transfer
        receiver_conn.send(b"C").await.unwrap();

        // YMODEM sends this as one 1024-byte STX block.
        let mut data_block = vec![0u8; 1029];
        receiver_conn.read_exact(&mut data_block).await.unwrap();
        assert_eq!(data_block[0], 0x02);
        receiver_conn.send(&[ACK]).await.unwrap();

        // EOT
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[NAK]).await.unwrap();
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // End of batch
        receiver_conn.send(b"C").await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();
    });

    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}

fn crc_block(header: u8, block_num: u8, payload: &[u8], block_len: usize, pad: u8) -> Vec<u8> {
    let mut data = payload.to_vec();
    data.resize(block_len, pad);
    let mut packet = vec![header, block_num, !block_num];
    packet.extend_from_slice(&data);
    let crc = crc16(&data);
    packet.push((crc >> 8) as u8);
    packet.push((crc & 0xFF) as u8);
    packet
}

fn ymodem_header(name: &str, size: usize) -> Vec<u8> {
    let mut info = Vec::new();
    info.extend_from_slice(name.as_bytes());
    info.push(0);
    info.extend_from_slice(format!("{size} 1700000000 100644").as_bytes());
    crc_block(SOH, 0, &info, 128, 0)
}

/// Section 5: the stated length lets the receiver discard the padding, preserving
/// the exact contents even when the file itself ends in 0x1A.
#[tokio::test]
async fn the_stated_length_keeps_a_file_that_ends_in_the_pad_byte() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let payload = vec![b'a', b'b', 0x1A, 0x1A];

    let header = ymodem_header("padded.bin", payload.len());
    let data_block = crc_block(SOH, 1, &payload, 128, 0x1A);

    tokio::spawn(async move {
        let mut buf = [0u8; 1];
        sender_conn.read(&mut buf).await.unwrap();

        sender_conn.send(&header).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap(); // ACK
        sender_conn.read(&mut buf).await.unwrap(); // C

        sender_conn.send(&data_block).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap(); // ACK

        sender_conn.send(&[EOT]).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap(); // NAK
        sender_conn.send(&[EOT]).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap(); // ACK
        sender_conn.read(&mut buf).await.unwrap(); // C

        sender_conn.send(&crc_block(SOH, 0, &[0], 128, 0)).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap(); // ACK
    });

    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let state = test_receiver(&mut receiver_conn, &mut protocol).await;

    let received = fs::read(&state.recieve_state.finished_files[0].1).unwrap();
    assert_eq!(received, payload, "trailing 0x1A bytes of the file itself must survive");
}

/// Section 6, figure 8: a streaming batch is driven by G, not by C.
#[tokio::test]
async fn a_streaming_batch_asks_for_the_next_block_with_g() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let payload = vec![3u8; 16];

    let header = ymodem_header("stream.bin", payload.len());
    let data_block = crc_block(SOH, 1, &payload, 128, 0x1A);

    let start_bytes = tokio::spawn(async move {
        let mut buf = [0u8; 1];
        sender_conn.read(&mut buf).await.unwrap();
        let initial = buf[0];

        sender_conn.send(&header).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap();
        let after_header = buf[0];

        // Streaming means the data block is not acknowledged.
        sender_conn.send(&data_block).await.unwrap();
        sender_conn.send(&[EOT]).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap(); // ACK
        assert_eq!(buf[0], ACK);
        sender_conn.read(&mut buf).await.unwrap();
        let next_file = buf[0];

        sender_conn.send(&crc_block(SOH, 0, &[0], 128, 0)).await.unwrap();
        sender_conn.read(&mut buf).await.unwrap();
        (initial, after_header, next_file)
    });

    let mut protocol = XYmodem::new(XYModemVariant::YModemG);
    let state = test_receiver(&mut receiver_conn, &mut protocol).await;

    let (initial, after_header, next_file) = start_bytes.await.unwrap();
    assert_eq!(initial, b'G', "the batch is opened with G");
    assert_eq!(after_header, b'G', "the file header is answered directly with G");
    assert_eq!(next_file, b'G', "the next file is requested with G");

    let received = fs::read(&state.recieve_state.finished_files[0].1).unwrap();
    assert_eq!(received, payload);
}

#[tokio::test]
async fn a_nak_retransmits_the_same_ymodem_file_header() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    let mut first_file = NamedTempFile::new().unwrap();
    first_file.write_all(b"first").unwrap();
    let mut second_file = NamedTempFile::new().unwrap();
    second_file.write_all(b"second").unwrap();

    let receiver = tokio::spawn(async move {
        receiver_conn.send(b"C").await.unwrap();

        let mut first_header = vec![0u8; 133];
        receiver_conn.read_exact(&mut first_header).await.unwrap();
        receiver_conn.send(&[NAK]).await.unwrap();

        let mut repeated_header = vec![0u8; 133];
        receiver_conn.read_exact(&mut repeated_header).await.unwrap();
        assert_eq!(repeated_header, first_header, "a NAK must not advance to the next file");

        receiver_conn.send(&[ACK, b'C']).await.unwrap();
        let mut block = vec![0u8; 133];
        receiver_conn.read_exact(&mut block).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();
        let mut eot = [0u8; 1];
        receiver_conn.read_exact(&mut eot).await.unwrap();
        receiver_conn.send(&[NAK]).await.unwrap();
        receiver_conn.read_exact(&mut eot).await.unwrap();
        receiver_conn.send(&[ACK, b'C']).await.unwrap();

        let mut second_header = vec![0u8; 133];
        receiver_conn.read_exact(&mut second_header).await.unwrap();
        assert_ne!(second_header, first_header, "the second file follows only after the first completed");

        receiver_conn.send(&[ACK, b'C']).await.unwrap();
        receiver_conn.read_exact(&mut block).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();
        receiver_conn.read_exact(&mut eot).await.unwrap();
        receiver_conn.send(&[NAK]).await.unwrap();
        receiver_conn.read_exact(&mut eot).await.unwrap();
        receiver_conn.send(&[ACK, b'C']).await.unwrap();

        receiver_conn.read_exact(&mut block).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();
    });

    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let state = test_sender(
        &mut sender_conn,
        &mut protocol,
        &[first_file.path().to_path_buf(), second_file.path().to_path_buf()],
    )
    .await;

    receiver.await.unwrap();
    assert_eq!(state.send_state.finished_files.len(), 2);
}

#[tokio::test]
async fn a_bad_ymodem_header_is_retried_from_its_soh() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let good_header = ymodem_header("retry.bin", 3);
    let mut bad_header = good_header.clone();
    *bad_header.last_mut().unwrap() ^= 1;
    let data = crc_block(SOH, 1, &[1, 2, 3], 128, 0x1A);

    tokio::spawn(async move {
        let mut response = [0u8; 1];
        sender_conn.read_exact(&mut response).await.unwrap();
        sender_conn.send(&bad_header).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], NAK);

        sender_conn.send(&good_header).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], ACK);
        sender_conn.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], b'C');

        sender_conn.send(&data).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        sender_conn.send(&[EOT]).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        sender_conn.send(&[EOT]).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        sender_conn.send(&crc_block(SOH, 0, &[0], 128, 0)).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
    });

    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let state = test_receiver(&mut receiver_conn, &mut protocol).await;
    assert_eq!(fs::read(&state.recieve_state.finished_files[0].1).unwrap(), [1, 2, 3]);
}

#[tokio::test]
async fn ymodem_g_sender_uses_the_streaming_handshake() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"streaming").unwrap();

    let receiver = tokio::spawn(async move {
        receiver_conn.send(b"G").await.unwrap();
        let mut block = vec![0u8; 133];
        receiver_conn.read_exact(&mut block).await.unwrap();
        assert_eq!(block[0], SOH);
        assert_eq!(block[1], 0);

        receiver_conn.send(b"G").await.unwrap();
        receiver_conn.read_exact(&mut block).await.unwrap();
        assert_eq!(block[1], 1);

        let mut eot = [0u8; 1];
        receiver_conn.read_exact(&mut eot).await.unwrap();
        assert_eq!(eot[0], EOT);
        receiver_conn.send(&[ACK, b'G']).await.unwrap();

        receiver_conn.read_exact(&mut block).await.unwrap();
        assert_eq!(block[1], 0);
        assert_eq!(block[3], 0);
        receiver_conn.send(&[ACK]).await.unwrap();
    });

    let mut protocol = XYmodem::new(XYModemVariant::YModemG);
    let state = test_sender(&mut sender_conn, &mut protocol, &[file.path().to_path_buf()]).await;
    receiver.await.unwrap();
    assert_eq!(state.send_state.finished_files.len(), 1);
}

#[tokio::test]
async fn a_ymodem_g_crc_error_aborts_with_can() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();
    let header = ymodem_header("broken.bin", 3);
    let mut data = crc_block(SOH, 1, &[1, 2, 3], 128, 0x1A);
    *data.last_mut().unwrap() ^= 1;

    let sender = tokio::spawn(async move {
        let mut response = [0u8; 1];
        sender_conn.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], b'G');
        sender_conn.send(&header).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], b'G');
        sender_conn.send(&data).await.unwrap();
        sender_conn.read_exact(&mut response).await.unwrap();
        response[0]
    });

    let mut protocol = XYmodem::new(XYModemVariant::YModemG);
    let mut state = protocol.initiate_recv(&mut receiver_conn).await.unwrap();
    let mut result = Ok(());
    while result.is_ok() {
        result = protocol.update_transfer(&mut receiver_conn, &mut state).await;
    }

    assert!(result.is_err());
    assert_eq!(sender.await.unwrap(), CAN);
}
