#![allow(dead_code)]

use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    time::Duration,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::ConnectionState;

use super::{Connection, ConnectionType};

mod negotiation;
mod telnet_cmd;
mod telnet_option;

use negotiation::Negotiation;

#[derive(Debug)]
enum ParserState {
    Data,
    Iac,
    Will,
    Wont,
    Do,
    Dont,
    SubCommand(i32),
}

mod terminal_type {
    pub const IS: u8 = 0x00;
    pub const SEND: u8 = 0x01;
    // pub const MAXLN: usize = 40;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TerminalEmulation {
    #[default]
    Ansi,
    Utf8Ansi,
    Avatar,
    Ascii,
    PETscii,
    ATAscii,
    ViewData,
    Mode7,
    Rip,
    Skypix,
    AtariST,
}

#[derive(Debug, Clone)]
pub struct TermCaps {
    pub window_size: (u16, u16), // width, height
    pub terminal: TerminalEmulation,
}

pub struct TelnetConnection {
    tcp_stream: TcpStream,
    caps: TermCaps,
    state: ParserState,
    read_buffer: Vec<u8>,
    is_server: bool,
    /// RFC 1143 negotiation state for the remote side (incoming WILL/WONT).
    remote_neg: HashMap<u8, Negotiation>,
    /// RFC 1143 negotiation state for the local side (incoming DO/DONT).
    local_neg: HashMap<u8, Negotiation>,
}

impl TelnetConnection {
    pub async fn open(addr: impl Into<String>, caps: TermCaps, timeout: Duration) -> crate::Result<Self> {
        let mut addr: String = addr.into();
        if !addr.contains(':') {
            addr.push_str(":23");
        }
        let result = tokio::time::timeout(timeout, TcpStream::connect(addr)).await;
        match result {
            Ok(tcp_stream) => match tcp_stream {
                Ok(tcp_stream) => Ok(Self {
                    tcp_stream,
                    caps,
                    state: ParserState::Data,
                    read_buffer: Vec::new(),
                    is_server: false,
                    remote_neg: HashMap::new(),
                    local_neg: HashMap::new(),
                }),
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    pub fn accept(tcp_stream: TcpStream) -> crate::Result<Self> {
        Ok(Self {
            tcp_stream,
            caps: TermCaps {
                window_size: (0, 0),
                terminal: TerminalEmulation::Ansi,
            },
            state: ParserState::Data,
            read_buffer: Vec::new(),
            is_server: true,
            remote_neg: HashMap::new(),
            local_neg: HashMap::new(),
        })
    }

    fn agree_remote(opt: u8, is_server: bool) -> bool {
        if is_server {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::TERMINAL_TYPE | telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE)
        } else {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::ECHO | telnet_option::SUPPRESS_GO_AHEAD)
        }
    }

    fn agree_local(opt: u8, is_server: bool) -> bool {
        if is_server {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::ECHO | telnet_option::SUPPRESS_GO_AHEAD)
        } else {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::TERMINAL_TYPE | telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE)
        }
    }

    async fn send_negotiation_reply(&mut self, reply: negotiation::Reply, opt: u8) -> io::Result<()> {
        use negotiation::Reply;
        match reply {
            Reply::Do => self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, opt)).await,
            Reply::Dont => self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DONT, opt)).await,
            Reply::Will => self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WILL, opt)).await,
            Reply::Wont => self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WONT, opt)).await,
        }
    }

    async fn parse(&mut self, data: &mut [u8]) -> io::Result<usize> {
        let mut write_ptr = 0;
        for i in 0..data.len() {
            let b = data[i];
            match self.state {
                ParserState::Data => {
                    if b == telnet_cmd::IAC {
                        self.state = ParserState::Iac;
                    } else {
                        data[write_ptr] = b;
                        write_ptr += 1;
                    }
                }

                ParserState::SubCommand(cmd) => {
                    match b {
                        telnet_cmd::IAC => {}
                        telnet_cmd::SE => {
                            self.state = ParserState::Data;
                        }
                        terminal_type::SEND => {
                            // Send
                            if cmd == telnet_option::TERMINAL_TYPE as i32 {
                                let mut buf = vec![telnet_cmd::IAC, telnet_cmd::SB, telnet_option::TERMINAL_TYPE, terminal_type::IS];

                                match self.caps.terminal {
                                    //  :TODO: Let's extend this to allow for some of the semi-standard BBS IDs, e.g. "xterm" (ANSI), "ansi-256-color", etc.
                                    TerminalEmulation::Ansi => buf.extend_from_slice(b"ANSI"),
                                    TerminalEmulation::Utf8Ansi => buf.extend_from_slice(b"UTF8ANSI"),
                                    TerminalEmulation::PETscii => buf.extend_from_slice(b"PETSCII"),
                                    TerminalEmulation::ATAscii => buf.extend_from_slice(b"ATASCII"),
                                    TerminalEmulation::ViewData => buf.extend_from_slice(b"VIEWDATA"),
                                    TerminalEmulation::Ascii => buf.extend_from_slice(b"RAW"),
                                    TerminalEmulation::Avatar => buf.extend_from_slice(b"AVATAR"),
                                    TerminalEmulation::Rip => buf.extend_from_slice(b"RIP"),
                                    TerminalEmulation::Skypix => buf.extend_from_slice(b"SKYPIX"),
                                    TerminalEmulation::AtariST => buf.extend_from_slice(b"ATARIST"),
                                    TerminalEmulation::Mode7 => buf.extend_from_slice(b"MODE7"),
                                }
                                buf.extend([telnet_cmd::IAC, telnet_cmd::SE]);
                                self.tcp_stream.write_all(&buf).await?;
                            }
                        }
                        24 => {
                            // Terminal type
                            self.state = ParserState::SubCommand(telnet_option::TERMINAL_TYPE as i32);
                        }
                        _ => {}
                    }
                }
                ParserState::Iac => match telnet_cmd::check(b) {
                    Ok(telnet_cmd::AYT) => {
                        self.state = ParserState::Data;
                        self.tcp_stream.write_all(&telnet_cmd::make_cmd(telnet_cmd::NOP)).await?;
                    }
                    Ok(telnet_cmd::SE | telnet_cmd::NOP | telnet_cmd::GA) => {
                        self.state = ParserState::Data;
                    }
                    Ok(telnet_cmd::IAC) => {
                        data[write_ptr] = 0xFF;
                        write_ptr += 1;
                        self.state = ParserState::Data;
                    }
                    Ok(telnet_cmd::WILL) => {
                        self.state = ParserState::Will;
                    }
                    Ok(telnet_cmd::WONT) => {
                        self.state = ParserState::Wont;
                    }
                    Ok(telnet_cmd::DO) => {
                        self.state = ParserState::Do;
                    }
                    Ok(telnet_cmd::DONT) => {
                        self.state = ParserState::Dont;
                    }
                    Ok(telnet_cmd::SB) => {
                        self.state = ParserState::SubCommand(-1);
                    }
                    Err(err) => {
                        log::error!("error parsing IAC: {}", err);
                        self.state = ParserState::Data;
                    }
                    Ok(cmd) => {
                        log::error!("unsupported IAC: {}", telnet_cmd::to_string(cmd));
                        self.state = ParserState::Data;
                    }
                },
                ParserState::Will => {
                    self.state = ParserState::Data;
                    let agree = Self::agree_remote(b, self.is_server);
                    let neg = self.remote_neg.get(&b).copied().unwrap_or_default();
                    let (new_neg, reply) = neg.on_will(agree);
                    self.remote_neg.insert(b, new_neg);
                    if let Some(reply) = reply {
                        self.send_negotiation_reply(reply, b).await?;
                    }
                }
                ParserState::Wont => {
                    self.state = ParserState::Data;
                    let neg = self.remote_neg.get(&b).copied().unwrap_or_default();
                    let (new_neg, reply) = neg.on_wont();
                    self.remote_neg.insert(b, new_neg);
                    if let Some(reply) = reply {
                        self.send_negotiation_reply(reply, b).await?;
                    }
                    log::info!("Wont {b:?}");
                }
                ParserState::Do => {
                    self.state = ParserState::Data;
                    let agree = Self::agree_local(b, self.is_server);
                    let neg = self.local_neg.get(&b).copied().unwrap_or_default();
                    let (new_neg, reply) = neg.on_do(agree);
                    self.local_neg.insert(b, new_neg);
                    let has_reply = reply.is_some();
                    if let Some(reply) = reply {
                        self.send_negotiation_reply(reply, b).await?;
                    }
                    if b == telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE && has_reply {
                        let mut buf: Vec<u8> = telnet_cmd::make_cmd_with_option(telnet_cmd::SB, telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE).to_vec();
                        buf.extend(self.caps.window_size.0.to_be_bytes());
                        buf.extend(self.caps.window_size.1.to_be_bytes());
                        buf.push(telnet_cmd::IAC);
                        buf.push(telnet_cmd::SE);
                        self.tcp_stream.write_all(&buf).await?;
                    }
                }
                ParserState::Dont => {
                    self.state = ParserState::Data;
                    let neg = self.local_neg.get(&b).copied().unwrap_or_default();
                    let (new_neg, reply) = neg.on_dont();
                    self.local_neg.insert(b, new_neg);
                    if let Some(reply) = reply {
                        self.send_negotiation_reply(reply, b).await?;
                    }
                    log::info!("Dont {b:?}");
                }
            }
        }
        Ok(write_ptr)
    }
}

#[async_trait]
impl Connection for TelnetConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Telnet
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First, check if we have buffered data from a previous poll
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // No buffered data, read from the stream
        match self.tcp_stream.read(buf).await {
            Ok(size) => {
                let result = self.parse(&mut buf[0..size]).await?;
                Ok(result)
            }
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    log::error!("telnet error - connection aborted.");
                    return Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into());
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("Error {:?} reading from SSH connection: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        // Try to read data to check connection status
        let mut buf = [0u8; 256]; // Use a reasonable buffer size
        match self.tcp_stream.try_read(&mut buf) {
            Ok(0) => {
                // A successful read of 0 bytes means the connection was closed cleanly
                Ok(ConnectionState::Disconnected)
            }
            Ok(n) => {
                // We got data - parse it and store the result in our buffer
                let parsed_len = self.parse(&mut buf[..n]).await?;
                if parsed_len > 0 {
                    // Store the parsed data in our internal buffer for later reading
                    self.read_buffer.extend_from_slice(&buf[..parsed_len]);
                }
                Ok(ConnectionState::Connected)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No data available, but connection is still open
                Ok(ConnectionState::Connected)
            }
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset | ErrorKind::NotConnected | ErrorKind::BrokenPipe | ErrorKind::UnexpectedEof
                ) =>
            {
                // These errors indicate the connection is definitely closed
                log::debug!("Telnet connection closed: {:?}", e);
                Ok(ConnectionState::Disconnected)
            }
            Err(e) => {
                // Other errors might be temporary, log them but assume connection is still valid
                log::warn!("Telnet poll error: {:?}", e);
                Err(Box::new(e))
            }
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First, check if we have buffered data from a previous poll
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // No buffered data, try to read from the stream
        match self.tcp_stream.try_read(buf) {
            Ok(size) => {
                let result = self.parse(&mut buf[0..size]).await?;
                Ok(result)
            }
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    log::error!("telnet error - connection aborted.");
                    return Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into());
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("Error {:?} reading from SSH connection: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        let mut dst = Vec::new();
        for b in buf {
            if *b == telnet_cmd::IAC {
                dst.extend_from_slice(&[telnet_cmd::IAC, telnet_cmd::IAC]);
            } else {
                dst.push(*b);
            }
        }
        self.tcp_stream.write_all(&dst).await?;
        Ok(())
    }
    async fn shutdown(&mut self) -> crate::Result<()> {
        self.tcp_stream.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agree_remote(opt: u8, is_server: bool) -> bool {
        if is_server {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::TERMINAL_TYPE | telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE)
        } else {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::ECHO | telnet_option::SUPPRESS_GO_AHEAD)
        }
    }

    fn agree_local(opt: u8, is_server: bool) -> bool {
        if is_server {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::ECHO | telnet_option::SUPPRESS_GO_AHEAD)
        } else {
            matches!(opt, telnet_option::TRANSMIT_BINARY | telnet_option::TERMINAL_TYPE | telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE)
        }
    }

    #[test]
    fn agree_remote_directionality() {
        // Client: wants server to do BINARY, ECHO, SGA; refuses TTYPE, NAWS
        assert!(agree_remote(telnet_option::TRANSMIT_BINARY, false));
        assert!(agree_remote(telnet_option::ECHO, false));
        assert!(agree_remote(telnet_option::SUPPRESS_GO_AHEAD, false));
        assert!(!agree_remote(telnet_option::TERMINAL_TYPE, false));
        assert!(!agree_remote(telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE, false));

        // Server: wants client to do BINARY, TTYPE, NAWS; refuses ECHO, SGA
        assert!(agree_remote(telnet_option::TRANSMIT_BINARY, true));
        assert!(agree_remote(telnet_option::TERMINAL_TYPE, true));
        assert!(agree_remote(telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE, true));
        assert!(!agree_remote(telnet_option::ECHO, true));
        assert!(!agree_remote(telnet_option::SUPPRESS_GO_AHEAD, true));
    }

    #[test]
    fn agree_local_directionality() {
        // Client: will do BINARY, TTYPE, NAWS; refuses ECHO, SGA
        assert!(agree_local(telnet_option::TRANSMIT_BINARY, false));
        assert!(agree_local(telnet_option::TERMINAL_TYPE, false));
        assert!(agree_local(telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE, false));
        assert!(!agree_local(telnet_option::ECHO, false));
        assert!(!agree_local(telnet_option::SUPPRESS_GO_AHEAD, false));

        // Server: will do BINARY, ECHO, SGA; refuses TTYPE, NAWS
        assert!(agree_local(telnet_option::TRANSMIT_BINARY, true));
        assert!(agree_local(telnet_option::ECHO, true));
        assert!(agree_local(telnet_option::SUPPRESS_GO_AHEAD, true));
        assert!(!agree_local(telnet_option::TERMINAL_TYPE, true));
        assert!(!agree_local(telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE, true));
    }
}
