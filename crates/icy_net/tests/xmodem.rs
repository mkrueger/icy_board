/*
use std::{fs, io::Write, path::PathBuf};

use icy_net::protocol::{XYModemVariant, XYmodem};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

use crate::test_connection::{test_receiver, test_sender, TestConnection};

mod test_connection;

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;

#[test]
fn test_send_xmodem_128block_checksum() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let mut data = vec![1u8, 2, 5, 10];
    test_connection.read_data.extend(&[
        NAK, ACK, ACK, // ACK EOT
    ]);
    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut test_connection, &mut protocol, &[PathBuf::from(named_temp.path())]);

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.finished_files[0].1, named_temp.path());
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);

    // construct result
    let mut result = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    result.extend_from_slice(&data);
    result.push(0xAA); // CHECKSUM
    result.push(EOT);
    assert_eq!(result, test_connection.write_data);
}

#[test]
fn test_send_xmodem_128block_crc16() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let mut data = vec![1u8, 2, 5, 10];
    test_connection.read_data.extend(&[
        b'C', ACK, ACK, // ACK EOT
    ]);
    let mut named_temp = NamedTempFile::new().unwrap();
    named_temp.as_file_mut().write_all(&data).unwrap();
    let state = test_sender(&mut test_connection, &mut protocol, &[PathBuf::from(named_temp.path())]);

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.finished_files[0].1, named_temp.path());
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);

    // construct result
    let mut result = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    result.extend_from_slice(&data);
    result.push(150); // CRC16 CHECKSUM
    result.push(207); // CRC16 CHECKSUM
    result.push(EOT);
    assert_eq!(result, test_connection.write_data);
}

#[test]
fn test_recv_xmodem_128block_checksum() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::XModem);
    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();

    let mut result = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    result.extend_from_slice(&data);
    result.push(0xAA); // CHECKSUM
    result.push(EOT);
    test_connection.read_data = result;

    let state = test_receiver(&mut test_connection, &mut protocol);

    assert_eq!(test_connection.write_data, &[NAK, ACK, ACK]);

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    assert_eq!(loaded_data, orig_data);
}

#[test]
fn test_recv_xmodem1k_128block_crc16() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);
    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();

    let mut result = vec![SOH, 0x01, 0xFE];
    data.resize(128, 0x1A);
    result.extend_from_slice(&data);
    result.push(150); // CRC16 CHECKSUM
    result.push(207); // CRC16 CHECKSUM
    result.push(EOT);
    test_connection.read_data = result;

    let state = test_receiver(&mut test_connection, &mut protocol);
    assert_eq!(test_connection.write_data, &[b'C', ACK, ACK,]);

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    assert_eq!(loaded_data, orig_data);
}
*/
