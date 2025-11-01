use std::{fs, io::Write, path::PathBuf};

use icy_net::{
    Connection,
    protocol::{XYModemVariant, XYmodem},
};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

mod test_connection;
use test_connection::{TestConnection, test_receiver, test_sender};

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;

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
        receiver_conn.send(&[b'C']).await.unwrap();

        // Read header block
        let mut buf = vec![0u8; 133];
        receiver_conn.read(&mut buf).await.unwrap();
        received.extend_from_slice(&buf);
        receiver_conn.send(&[ACK]).await.unwrap();

        // Send 'C' for data transfer
        receiver_conn.send(&[b'C']).await.unwrap();

        // Read data block
        receiver_conn.read(&mut buf).await.unwrap();
        received.extend_from_slice(&buf);
        receiver_conn.send(&[ACK]).await.unwrap();

        // Read EOT
        let mut eot_buf = [0u8; 1];
        receiver_conn.read(&mut eot_buf).await.unwrap();
        assert_eq!(eot_buf[0], EOT);
        received.push(EOT);
        receiver_conn.send(&[ACK]).await.unwrap();

        // Send 'C' for end of batch
        receiver_conn.send(&[b'C']).await.unwrap();

        // Read end-of-batch block (empty header)
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Verify the first part (actual file transfer) matches expected
        assert_eq!(received[..expected_clone.len()], expected_clone[..]);
    });

    let state = test_sender(&mut sender_conn, &mut protocol, &[temp_path.clone()]).await;

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
        receiver_conn.send(&[b'C']).await.unwrap();
        let mut buf = vec![0u8; 133];
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // First file data
        receiver_conn.send(&[b'C']).await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // First file EOT
        let mut eot_buf = [0u8; 1];
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Second file header
        receiver_conn.send(&[b'C']).await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Second file data
        receiver_conn.send(&[b'C']).await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Second file EOT
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // End of batch
        receiver_conn.send(&[b'C']).await.unwrap();
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
        receiver_conn.send(&[b'C']).await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // Request data transfer
        receiver_conn.send(&[b'C']).await.unwrap();

        // 3 data blocks (300 bytes = 3 * 128 byte blocks)
        for _ in 0..3 {
            receiver_conn.read(&mut buf).await.unwrap();
            receiver_conn.send(&[ACK]).await.unwrap();
        }

        // EOT
        receiver_conn.read(&mut eot_buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();

        // End of batch
        receiver_conn.send(&[b'C']).await.unwrap();
        receiver_conn.read(&mut buf).await.unwrap();
        receiver_conn.send(&[ACK]).await.unwrap();
    });

    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}
