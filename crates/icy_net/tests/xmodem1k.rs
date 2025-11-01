use std::{fs, io::Write, path::PathBuf};

use icy_net::{
    Connection,
    protocol::{XYModemVariant, XYmodem},
};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

mod test_connection;
use test_connection::{TestConnection, test_receiver, test_sender};

const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;

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
async fn test_send_xmodem1k() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Spawn receiver simulation
    tokio::spawn(async move {
        receiver_conn.send(&[b'C']).await.unwrap(); // Request CRC mode
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK data block
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK EOT
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);
    let data = vec![5; 900];

    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(named_temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.finished_files[0].1, named_temp.path());
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}

#[tokio::test]
async fn test_recv_xmodem1k() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    let orig_data = vec![5; 900];
    let mut data = orig_data.clone();

    let mut packet = vec![STX, 0x01, 0xFE];
    data.resize(1024, 0x1A);
    packet.extend_from_slice(&data);

    // Calculate correct CRC16
    let crc = crc16(&data);
    packet.push((crc >> 8) as u8); // CRC high byte
    packet.push((crc & 0xFF) as u8); // CRC low byte

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

    // XModem pads files, so the received file will be 1024 bytes
    // We should compare only the first 900 bytes
    assert_eq!(loaded_data[..orig_data.len()], orig_data[..]);
}

#[tokio::test]
async fn test_send_xmodem1k_multiple_blocks() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Create data that spans multiple 1K blocks (2500 bytes = 3 blocks)
    let data: Vec<u8> = (0..2500).map(|i| (i % 256) as u8).collect();

    // Spawn receiver simulation
    tokio::spawn(async move {
        receiver_conn.send(&[b'C']).await.unwrap(); // Request CRC mode
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK block 1
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK block 2
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK block 3
        receiver_conn.send(&[ACK]).await.unwrap(); // ACK EOT
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);

    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut sender_conn, &mut protocol, &[PathBuf::from(named_temp.path())]).await;

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);
}

#[tokio::test]
async fn test_recv_xmodem1k_exact_block() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Create data that's exactly 1024 bytes
    let orig_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    let mut packet = vec![STX, 0x01, 0xFE];
    packet.extend_from_slice(&orig_data);

    // Calculate correct CRC16
    let crc = crc16(&orig_data);
    packet.push((crc >> 8) as u8); // CRC high byte
    packet.push((crc & 0xFF) as u8); // CRC low byte

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
    assert_eq!(state.recieve_state.total_bytes_transfered, 1024);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    // File should be exactly 1024 bytes
    assert_eq!(loaded_data.len(), 1024);
    assert_eq!(loaded_data, orig_data);
}
