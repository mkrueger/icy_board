/*
use std::{fs, io::Write, path::PathBuf};

use icy_net::protocol::{XYModemVariant, XYmodem};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

use crate::test_connection::{test_receiver, test_sender, TestConnection};

mod test_connection;

const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;

#[test]
fn test_send_xmodem1k() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);
    let mut data = vec![5; 900];
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
    let mut result = vec![STX, 0x01, 0xFE];
    data.resize(1024, 0x1A);
    result.extend_from_slice(&data);
    result.push(184); // CHECKSUM
    result.push(85); // CHECKSUM
    result.push(EOT);

    assert_eq!(result, test_connection.write_data);
}

#[test]
fn test_recv_xmodem1k() {
    let mut test_connection = TestConnection::default();
    let mut protocol = XYmodem::new(XYModemVariant::XModem1k);
    let orig_data = vec![5; 900];
    let mut data = orig_data.clone();

    let mut result = vec![STX, 0x01, 0xFE];
    data.resize(1024, 0x1A);
    result.extend_from_slice(&data);
    result.push(184); // CHECKSUM
    result.push(85); // CHECKSUM
    result.push(EOT);
    test_connection.read_data = result;

    let state = test_receiver(&mut test_connection, &mut protocol);

    assert_eq!(test_connection.write_data, &[b'C', ACK, ACK]);

    assert_eq!(state.recieve_state.finished_files.len(), 1);
    assert_eq!(state.recieve_state.total_bytes_transfered, data.len() as u64);

    let loaded_data = fs::read(&state.recieve_state.finished_files[0].1).unwrap();

    assert_eq!(loaded_data, orig_data);
}
*/
