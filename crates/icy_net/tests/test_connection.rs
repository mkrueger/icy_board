/*
#![allow(dead_code)]
use icy_net::{protocol::TransferState, Connection, ConnectionType};
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Default)]
pub struct TestConnection {
    pub read_data: Vec<u8>,
    pub write_data: Vec<u8>,
}

impl Connection for TestConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }
}

impl Read for TestConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.read_data.len().min(buf.len());
        buf[0..len].clone_from_slice(&self.read_data[0..len]);
        self.read_data.drain(0..len);
        Ok(len)
    }
}

impl Write for TestConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_data.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn test_sender(con: &mut TestConnection, send: &mut dyn icy_net::protocol::Protocol, files: &[PathBuf]) -> TransferState {
    let mut state = send.initiate_send(con, files).expect("error.");

    while !state.is_finished {
        send.update_transfer(con, &mut state).expect("error.");
    }

    state
}

pub fn test_receiver(con: &mut TestConnection, send: &mut dyn icy_net::protocol::Protocol) -> TransferState {
    let mut state = send.initiate_recv(con).expect("error.");

    while !state.is_finished {
        send.update_transfer(con, &mut state).unwrap();
    }

    state
}
*/
