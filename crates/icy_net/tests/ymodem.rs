/*
use std::{fs, path::PathBuf};

use icy_net::protocol::{XYModemVariant, XYmodem};
//use pretty_assertions::assert_eq;

use crate::test_connection::{test_receiver, test_sender, TestConnection};

mod test_connection;

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;

#[test]
fn test_send_ymodem() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let mut data = vec![1u8, 2, 5, 10];
    test_connection.read_data.extend(&[b'C', ACK, b'C', ACK, ACK, ACK]);
    let file_name = "foo.bar";

    fs::write(file_name, &data).unwrap();
    let state = test_sender(&mut test_connection, &mut protocol, &[PathBuf::from(file_name)]);

    assert_eq!(state.send_state.finished_files.len(), 1);
    assert_eq!(state.send_state.total_bytes_transfered, data.len() as u64);

    // construct result
    let mut result = vec![SOH, 0x00, 0xFF];
    result.extend_from_slice(file_name.as_bytes());
    result.extend_from_slice(&[0, b'4']); // length

    result.extend_from_slice(vec![0; 128 - file_name.len() - 2].as_slice());
    result.extend_from_slice(&[108, 107]); // CHECKSUM

    result.extend_from_slice(&[SOH, 0x01, 0xFE]);
    data.resize(128, 0x1A);
    result.extend_from_slice(&data);
    result.extend_from_slice(&[150, 207]); // CHECKSUM
    result.push(EOT);

    assert_eq!(result, test_connection.write_data);
    fs::remove_file(file_name).unwrap();
}

#[test]
fn test_recv_ymodem() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::YModem);
    let orig_data = vec![1u8, 2, 5, 10];
    let mut data = orig_data.clone();
    let file_name = "foo.bar";

    let mut result = vec![SOH, 0x00, 0xFF];
    result.extend_from_slice(file_name.as_bytes());
    result.extend_from_slice(&[0, b'4']); // length

    result.extend_from_slice(vec![0; 128 - file_name.len() - 2].as_slice());
    result.extend_from_slice(&[108, 107]); // CHECKSUM

    result.extend_from_slice(&[SOH, 0x01, 0xFE]);
    data.resize(128, 0x1A);
    result.extend_from_slice(&data);
    result.extend_from_slice(&[150, 207]); // CHECKSUM
    result.push(EOT); // -> NACK
    result.push(EOT); // -> ACK

    // No next file:
    result.extend_from_slice(&[SOH, 0x00, 0xFF]);
    result.extend_from_slice(vec![0; 128].as_slice());
    result.extend_from_slice(&[0, 0]);

    test_connection.read_data = result;

    let state = test_receiver(&mut test_connection, &mut protocol);

    assert_eq!(test_connection.write_data, &[b'C', ACK, b'C', ACK, NAK, ACK, b'C', ACK]);

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    assert_eq!(loaded_data, orig_data);
}
*/
