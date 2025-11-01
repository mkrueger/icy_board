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

#[tokio::test]
async fn test_send_xmodem_128block_checksum() {
    // Create a mock connection with pre-programmed responses
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Simulate receiver responses
    tokio::spawn(async move {
        receiver_conn.send(&[NAK]).await.unwrap(); // Request checksum mode
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK the data block
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK the EOT
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let data = vec![1u8, 2, 5, 10];

    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(named_temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.finished_files[0].1, named_temp.path());
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}

#[tokio::test]
async fn test_send_xmodem_128block_crc16() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Simulate receiver responses
    tokio::spawn(async move {
        receiver_conn.send(&[b'C']).await.unwrap(); // Request CRC mode
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK the data block
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK the EOT
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let data = vec![1u8, 2, 5, 10];

    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(named_temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.finished_files[0].1, named_temp.path());
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}

#[tokio::test]
async fn test_recv_xmodem_128block_checksum() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Prepare the data to be "sent" by the mock sender
    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();

    let mut packet = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    packet.extend_from_slice(&data);
    packet.push(0xAA); // CHECKSUM

    // Spawn sender simulation
    tokio::spawn(async move {
        // Wait for NAK from receiver
        let mut buf = [0u8; 1];
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], NAK);

        // Send the data packet
        sender_conn.send(&packet).await.unwrap();

        // Wait for ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);

        // Send EOT
        sender_conn.send(&[EOT]).await.unwrap();

        // Wait for final ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let state = test_receiver(&mut receiver_conn, &mut protocol).await;

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    // XModem pads files to block size, so we need to compare only the original data length
    assert_eq!(loaded_data[..orig_data.len()], orig_data[..]);
}

#[tokio::test]
async fn test_recv_xmodem1k_128block_crc16() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();

    let mut packet = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    packet.extend_from_slice(&data);
    packet.push(150); // CRC16 CHECKSUM
    packet.push(207); // CRC16 CHECKSUM

    // Spawn sender simulation
    tokio::spawn(async move {
        // Wait for C from receiver
        let mut buf = [0u8; 1];
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], b'C');

        // Send the data packet
        sender_conn.send(&packet).await.unwrap();

        // Wait for ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);

        // Send EOT
        sender_conn.send(&[EOT]).await.unwrap();

        // Wait for final ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);
    let state = test_receiver(&mut receiver_conn, &mut protocol).await;

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    // XModem pads files to block size, so we need to compare only the original data length
    assert_eq!(loaded_data[..orig_data.len()], orig_data[..]);
}

#[tokio::test]
async fn test_send_xmodem_multiple_blocks() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Simulate receiver responses
    tokio::spawn(async move {
        receiver_conn.send(&[NAK]).await.unwrap(); // Request checksum mode
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK block 1
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK block 2
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK EOT
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);

    // Create data that spans multiple blocks (200 bytes = 2 blocks)
    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();

    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(named_temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}

#[tokio::test]
async fn test_xmodem1k_large_block() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Simulate receiver responses
    tokio::spawn(async move {
        receiver_conn.send(&[b'C']).await.unwrap(); // Request CRC mode
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK block
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK EOT
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);

    // Create data that fits in one 1K block
    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(named_temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}
