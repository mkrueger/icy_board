#![allow(dead_code)]

use std::{
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

use negotiation::{Negotiation, Reply};

#[derive(Debug, Clone, Copy)]
enum ParserState {
    Data,
    Iac,
    Will,
    Wont,
    Do,
    Dont,
    /// Waiting for the byte that says which option is being subnegotiated.
    SubOption,
    SubCommand(u8),
    SubCommandIac(u8),
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

fn terminal_name(terminal: TerminalEmulation) -> &'static [u8] {
    match terminal {
        TerminalEmulation::Ansi => b"ANSI",
        TerminalEmulation::Utf8Ansi => b"UTF8ANSI",
        TerminalEmulation::PETscii => b"PETSCII",
        TerminalEmulation::ATAscii => b"ATASCII",
        TerminalEmulation::ViewData => b"VIEWDATA",
        TerminalEmulation::Ascii => b"RAW",
        TerminalEmulation::Avatar => b"AVATAR",
        TerminalEmulation::Rip => b"RIP",
        TerminalEmulation::Skypix => b"SKYPIX",
        TerminalEmulation::AtariST => b"ATARIST",
        TerminalEmulation::Mode7 => b"MODE7",
    }
}

/// Anything a plain terminal answers with - xterm, vt100, linux - is an ANSI
/// terminal as far as a board is concerned, so only the BBS names are matched.
fn terminal_from_name(name: &[u8]) -> TerminalEmulation {
    let name = String::from_utf8_lossy(name).trim().to_ascii_uppercase();
    match name.as_str() {
        "UTF8ANSI" | "UTF-8" => TerminalEmulation::Utf8Ansi,
        "PETSCII" => TerminalEmulation::PETscii,
        "ATASCII" => TerminalEmulation::ATAscii,
        "VIEWDATA" => TerminalEmulation::ViewData,
        "RAW" | "DUMB" | "UNKNOWN" | "NETWORK" => TerminalEmulation::Ascii,
        "AVATAR" => TerminalEmulation::Avatar,
        "RIP" => TerminalEmulation::Rip,
        "SKYPIX" => TerminalEmulation::Skypix,
        "ATARIST" => TerminalEmulation::AtariST,
        "MODE7" => TerminalEmulation::Mode7,
        _ => TerminalEmulation::Ansi,
    }
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
    sub_buffer: Vec<u8>,
    /// Which end of the line we are: it decides which options we offer and which
    /// ones we let the peer turn on.
    is_server: bool,
    /// What the peer may enable for itself, indexed by option.
    remote: [Negotiation; 256],
    /// What we have agreed to enable on our side, indexed by option.
    local: [Negotiation; 256],
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
                Ok(tcp_stream) => Ok(Self::new(tcp_stream, caps, false)),
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    pub fn accept(tcp_stream: TcpStream) -> crate::Result<Self> {
        Ok(Self::new(
            tcp_stream,
            TermCaps {
                window_size: (0, 0),
                terminal: TerminalEmulation::Ansi,
            },
            true,
        ))
    }

    fn new(tcp_stream: TcpStream, caps: TermCaps, is_server: bool) -> Self {
        Self {
            tcp_stream,
            caps,
            state: ParserState::Data,
            read_buffer: Vec::new(),
            sub_buffer: Vec::new(),
            is_server,
            remote: [Negotiation::default(); 256],
            local: [Negotiation::default(); 256],
        }
    }

    /// What the peer told us about itself, as far as it got negotiated.
    pub fn caps(&self) -> &TermCaps {
        &self.caps
    }

    /// Whether we let the peer switch an option on for itself.
    fn accept_from_peer(&self, option: u8) -> bool {
        if self.is_server {
            // A board wants to know the caller's terminal and window, and it wants
            // eight bit clean transfers. Echo and go ahead are its own job.
            matches!(
                option,
                telnet_option::TRANSMIT_BINARY | telnet_option::TERMINAL_TYPE | telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE
            )
        } else {
            matches!(option, telnet_option::TRANSMIT_BINARY | telnet_option::ECHO | telnet_option::SUPPRESS_GO_AHEAD)
        }
    }

    /// Whether we are willing to switch an option on at our end.
    fn accept_for_us(&self, option: u8) -> bool {
        if self.is_server {
            matches!(option, telnet_option::TRANSMIT_BINARY | telnet_option::ECHO | telnet_option::SUPPRESS_GO_AHEAD)
        } else {
            matches!(
                option,
                telnet_option::TRANSMIT_BINARY | telnet_option::TERMINAL_TYPE | telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE
            )
        }
    }

    async fn send(&mut self, command: u8, option: u8) -> io::Result<()> {
        self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(command, option)).await
    }

    async fn send_reply(&mut self, reply: Reply, positive: u8, negative: u8, option: u8) -> io::Result<()> {
        match reply {
            Reply::Accept => self.send(positive, option).await,
            Reply::Refuse => self.send(negative, option).await,
        }
    }

    async fn send_subnegotiation(&mut self, option: u8, payload: &[u8]) -> io::Result<()> {
        let mut buf: Vec<u8> = telnet_cmd::make_cmd_with_option(telnet_cmd::SB, option).to_vec();
        buf.extend_from_slice(payload);
        buf.push(telnet_cmd::IAC);
        buf.push(telnet_cmd::SE);
        self.tcp_stream.write_all(&buf).await
    }

    async fn send_window_size(&mut self) -> io::Result<()> {
        // Note: big endian bytes are correct.
        let mut payload = self.caps.window_size.0.to_be_bytes().to_vec();
        payload.extend(self.caps.window_size.1.to_be_bytes());
        self.send_subnegotiation(telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE, &payload).await
    }

    /// Runs when an option has just switched on, for the ones that only become
    /// useful once something is sent over them.
    async fn on_enabled(&mut self, option: u8, remote_side: bool) -> io::Result<()> {
        match (option, remote_side) {
            (telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE, false) => self.send_window_size().await,
            (telnet_option::TERMINAL_TYPE, true) => self.send_subnegotiation(telnet_option::TERMINAL_TYPE, &[terminal_type::SEND]).await,
            _ => Ok(()),
        }
    }

    async fn on_subnegotiation(&mut self, option: u8) -> io::Result<()> {
        let payload = std::mem::take(&mut self.sub_buffer);
        match option {
            telnet_option::TERMINAL_TYPE => match payload.first() {
                Some(&terminal_type::SEND) => {
                    let mut buf = vec![terminal_type::IS];
                    buf.extend_from_slice(terminal_name(self.caps.terminal));
                    self.send_subnegotiation(telnet_option::TERMINAL_TYPE, &buf).await?;
                }
                Some(&terminal_type::IS) => {
                    self.caps.terminal = terminal_from_name(&payload[1..]);
                }
                _ => {}
            },
            telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE => {
                if payload.len() >= 4 {
                    let width = u16::from_be_bytes([payload[0], payload[1]]);
                    let height = u16::from_be_bytes([payload[2], payload[3]]);
                    // A zero means "no opinion", so it must not overwrite what we have.
                    if width > 0 && height > 0 {
                        self.caps.window_size = (width, height);
                    }
                }
            }
            _ => {
                log::info!("ignored subnegotiation for {}", telnet_option::to_string(option));
            }
        }
        Ok(())
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

                ParserState::SubOption => {
                    self.sub_buffer.clear();
                    self.state = ParserState::SubCommand(b);
                }

                ParserState::SubCommand(option) => {
                    if b == telnet_cmd::IAC {
                        self.state = ParserState::SubCommandIac(option);
                    } else {
                        self.sub_buffer.push(b);
                    }
                }

                ParserState::SubCommandIac(option) => match b {
                    telnet_cmd::SE => {
                        self.state = ParserState::Data;
                        self.on_subnegotiation(option).await?;
                    }
                    telnet_cmd::IAC => {
                        self.sub_buffer.push(telnet_cmd::IAC);
                        self.state = ParserState::SubCommand(option);
                    }
                    _ => {
                        self.state = ParserState::SubCommand(option);
                    }
                },
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
                        self.state = ParserState::SubOption;
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
                    let agree = self.accept_from_peer(b);
                    if !agree {
                        log::warn!("declined will option {}", telnet_option::to_string(b));
                    }
                    if let Some(reply) = self.remote[b as usize].on_positive(agree) {
                        self.send_reply(reply, telnet_cmd::DO, telnet_cmd::DONT, b).await?;
                    }
                    if self.remote[b as usize].is_enabled() {
                        self.on_enabled(b, true).await?;
                    }
                }
                ParserState::Wont => {
                    self.state = ParserState::Data;
                    log::info!("Wont {}", telnet_option::to_string(b));
                    if let Some(reply) = self.remote[b as usize].on_negative() {
                        self.send_reply(reply, telnet_cmd::DO, telnet_cmd::DONT, b).await?;
                    }
                }
                ParserState::Do => {
                    self.state = ParserState::Data;
                    let agree = self.accept_for_us(b);
                    if !agree {
                        log::warn!("declined do option {}", telnet_option::to_string(b));
                    }
                    if let Some(reply) = self.local[b as usize].on_positive(agree) {
                        self.send_reply(reply, telnet_cmd::WILL, telnet_cmd::WONT, b).await?;
                    }
                    if self.local[b as usize].is_enabled() {
                        self.on_enabled(b, false).await?;
                    }
                }
                ParserState::Dont => {
                    self.state = ParserState::Data;
                    log::info!("Dont {}", telnet_option::to_string(b));
                    if let Some(reply) = self.local[b as usize].on_negative() {
                        self.send_reply(reply, telnet_cmd::WILL, telnet_cmd::WONT, b).await?;
                    }
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
    use tokio::net::TcpListener;

    const IAC: u8 = telnet_cmd::IAC;
    const SB: u8 = telnet_cmd::SB;
    const SE: u8 = telnet_cmd::SE;
    const WILL: u8 = telnet_cmd::WILL;
    const WONT: u8 = telnet_cmd::WONT;
    const DO: u8 = telnet_cmd::DO;
    const DONT: u8 = telnet_cmd::DONT;
    const ECHO: u8 = telnet_option::ECHO;
    const TTYPE: u8 = telnet_option::TERMINAL_TYPE;
    const NAWS: u8 = telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE;

    /// A connection wired to a loopback socket, so what the parser answers can be
    /// read back the way the peer would see it.
    async fn connect(is_server: bool, caps: TermCaps) -> (TelnetConnection, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
        let (near, _) = listener.accept().await.unwrap();
        (TelnetConnection::new(near, caps, is_server), peer)
    }

    async fn board() -> (TelnetConnection, TcpStream) {
        connect(
            true,
            TermCaps {
                window_size: (0, 0),
                terminal: TerminalEmulation::Ansi,
            },
        )
        .await
    }

    /// Feeds bytes to the parser and returns the payload it kept and everything it
    /// wrote back.
    async fn feed(connection: &mut TelnetConnection, peer: &mut TcpStream, input: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut data = input.to_vec();
        let len = connection.parse(&mut data).await.unwrap();
        data.truncate(len);

        let mut answer = Vec::new();
        let mut buf = [0; 256];
        while let Ok(Ok(read)) = tokio::time::timeout(Duration::from_millis(50), peer.read(&mut buf)).await {
            if read == 0 {
                break;
            }
            answer.extend_from_slice(&buf[..read]);
        }
        (data, answer)
    }

    #[tokio::test]
    async fn an_offer_is_answered_once_however_often_it_is_repeated() {
        let (mut board, mut peer) = board().await;
        let (_, answer) = feed(&mut board, &mut peer, &[IAC, WILL, NAWS, IAC, WILL, NAWS, IAC, WILL, NAWS]).await;
        // Answering each one is what used to bounce back and forth with a peer that
        // reads our answer as a fresh offer.
        assert_eq!(answer, vec![IAC, DO, NAWS]);
    }

    #[tokio::test]
    async fn a_board_takes_the_callers_side_of_the_negotiation() {
        let (mut board, mut peer) = board().await;
        let (_, answer) = feed(&mut board, &mut peer, &[IAC, WILL, ECHO, IAC, DO, ECHO, IAC, DO, NAWS]).await;
        // Echoing and sizing the window are the board's job and the caller's
        // respectively, so it declines both offers to swap them round.
        assert_eq!(answer, vec![IAC, DONT, ECHO, IAC, WILL, ECHO, IAC, WONT, NAWS]);
    }

    #[tokio::test]
    async fn a_board_asks_for_the_terminal_type_it_was_offered() {
        let (mut board, mut peer) = board().await;
        let (_, answer) = feed(&mut board, &mut peer, &[IAC, WILL, TTYPE]).await;
        assert_eq!(answer, vec![IAC, DO, TTYPE, IAC, SB, TTYPE, terminal_type::SEND, IAC, SE]);

        feed(
            &mut board,
            &mut peer,
            &[IAC, SB, TTYPE, terminal_type::IS, b'A', b'V', b'A', b'T', b'A', b'R', IAC, SE],
        )
        .await;
        assert_eq!(board.caps().terminal, TerminalEmulation::Avatar);
    }

    #[tokio::test]
    async fn a_terminal_name_a_board_has_no_mode_for_is_taken_as_ansi() {
        let (mut board, mut peer) = board().await;
        feed(
            &mut board,
            &mut peer,
            &[IAC, SB, TTYPE, terminal_type::IS, b'x', b't', b'e', b'r', b'm', IAC, SE],
        )
        .await;
        assert_eq!(board.caps().terminal, TerminalEmulation::Ansi);
    }

    #[tokio::test]
    async fn the_window_size_the_caller_reports_is_kept() {
        let (mut board, mut peer) = board().await;
        feed(&mut board, &mut peer, &[IAC, SB, NAWS, 0, 132, 0, 43, IAC, SE]).await;
        assert_eq!(board.caps().window_size, (132, 43));
    }

    #[tokio::test]
    async fn a_doubled_ff_inside_a_subnegotiation_is_a_width_and_not_an_end_marker() {
        let (mut board, mut peer) = board().await;
        feed(&mut board, &mut peer, &[IAC, SB, NAWS, 0, IAC, IAC, 0, 43, IAC, SE]).await;
        assert_eq!(board.caps().window_size, (255, 43));
    }

    #[tokio::test]
    async fn a_client_offers_the_window_size_it_was_asked_for() {
        let (mut client, mut peer) = connect(
            false,
            TermCaps {
                window_size: (80, 25),
                terminal: TerminalEmulation::Ansi,
            },
        )
        .await;
        let (_, answer) = feed(&mut client, &mut peer, &[IAC, DO, NAWS]).await;
        assert_eq!(answer, vec![IAC, WILL, NAWS, IAC, SB, NAWS, 0, 80, 0, 25, IAC, SE]);
    }

    #[tokio::test]
    async fn payload_around_a_command_survives() {
        let (mut board, mut peer) = board().await;
        let (data, _) = feed(&mut board, &mut peer, &[b'h', b'i', IAC, WILL, NAWS, b'!', IAC, IAC, b'?']).await;
        assert_eq!(data, b"hi!\xFF?");
    }
}
