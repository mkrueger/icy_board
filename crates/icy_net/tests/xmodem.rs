use std::{fs, io::Write, path::PathBuf};

use icy_net::{
    Connection,
    protocol::{XYModemVariant, XYmodem},
};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

mod test_connection;
use test_connection::TestConnection;

use crate::test_connection::{test_receiver, test_sender};

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

/// Test that after a successful XModem receive, the protocol is truly finished
/// and doesn't block waiting for more data.
/// Bug report: "Xmodem transfers (upon completion) make the program freeze"
#[tokio::test]
async fn test_xmodem_recv_completes_cleanly() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();

    let mut packet = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    packet.extend_from_slice(&data);
    packet.push(0xAA); // CHECKSUM

    // Spawn sender simulation
    let sender_handle = tokio::spawn(async move {
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

        // Sender is done - close connection
        sender_conn.shutdown().await.unwrap();
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);

    // Use timeout to detect if protocol hangs
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), test_receiver(&mut receiver_conn, &mut protocol)).await;

    assert!(result.is_ok(), "XModem receive should complete within timeout, not hang");
    let state = result.unwrap();

    assert!(state.is_finished, "Transfer should be marked as finished");
    assert_eq!(state.recieve_state.finished_files.len(), 1);

    // Wait for sender to complete
    sender_handle.await.unwrap();
}

/// Test that the connection remains usable after XModem transfer completes.
/// Bug report: "communication-wise with the BBS... all frozen"
#[tokio::test]
async fn test_connection_usable_after_xmodem_recv() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

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

        // After transfer, send some BBS data
        sender_conn.send(b"Download complete!\r\n").await.unwrap();
        sender_conn.send(b"Press any key...").await.unwrap();
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let state = test_receiver(&mut receiver_conn, &mut protocol).await;

    assert!(state.is_finished, "Transfer should be marked as finished");

    // Now try to read post-transfer data from the BBS
    // This should NOT hang
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut buf = [0u8; 100];
        receiver_conn.read(&mut buf).await
    })
    .await;

    assert!(result.is_ok(), "Reading after transfer should not hang");
    let bytes_read = result.unwrap().unwrap();
    assert!(bytes_read > 0, "Should receive post-transfer BBS data");
}

/// Test XModem receive with immediate BBS data following EOT+ACK
/// This simulates a real BBS that immediately sends data after the transfer
#[tokio::test]
async fn test_xmodem_recv_with_immediate_followup_data() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();

    let mut packet = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    packet.extend_from_slice(&data);
    packet.push(0xAA); // CHECKSUM

    // Spawn sender simulation - this time send BBS data immediately after EOT
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

        // Send EOT followed IMMEDIATELY by BBS text (no wait for ACK read)
        // This simulates a BBS that sends data right after EOT
        sender_conn.send(&[EOT]).await.unwrap();

        // Wait for final ACK
        sender_conn.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], ACK);

        // Immediately send BBS data (simulating fast BBS response)
        sender_conn.send(b"\r\nTransfer complete!\r\nMain Menu:\r\n").await.unwrap();
    });

    let mut protocol = XYmodem::new(XYModemVariant::XModem);

    // Receive with timeout
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), test_receiver(&mut receiver_conn, &mut protocol)).await;

    assert!(result.is_ok(), "XModem receive should complete within timeout");
    let state = result.unwrap();

    assert!(state.is_finished, "Transfer should be marked as finished");
    assert_eq!(state.recieve_state.finished_files.len(), 1);

    // Try to read the BBS menu that was sent after the transfer
    let mut buf = [0u8; 100];
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), receiver_conn.read(&mut buf)).await;

    assert!(result.is_ok(), "Should be able to read post-transfer data");
    let n = result.unwrap().unwrap();
    let received = String::from_utf8_lossy(&buf[..n]);
    assert!(received.contains("Transfer complete"), "Should receive BBS response: got '{}'", received);
}

/// Test that read_u8 doesn't hang when connection returns 0 bytes
/// This could happen with certain connection types
#[tokio::test]
async fn test_read_u8_with_delayed_data() {
    let (mut sender_conn, mut receiver_conn) = TestConnection::create_pair();

    // Spawn sender that sends data after a small delay
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        sender_conn.send(&[0x42]).await.unwrap();
    });

    // read_u8 should wait for data without spinning forever
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), receiver_conn.read_u8()).await;

    assert!(result.is_ok(), "read_u8 should complete when data arrives");
    assert_eq!(result.unwrap().unwrap(), 0x42);
}
