//! Minimal RLogin (remote login) client connection implementation.
//!
//! This module provides a simplified rlogin handshake and stream wrapper
//! suitable for vintage BBS integrations. It intentionally supports a
//! "swapped" mode where the order of username and password fields in the
//! initial handshake is reversed to accommodate certain legacy servers.
//!
//! Classic BSD rlogin sends:
//!     NUL <client-user> NUL <server-user> NUL <terminal/speed> NUL
//!
//! Some BBS variants repurpose these fields as:
//!     NUL <password> NUL <username> NUL <terminal/speed> NUL
//!
//! We support both layouts with the `swapped` flag:
//!   swapped = false  => password first, then username (BBS style default here)
//!   swapped = true   => username first, then password (reverse / alternate)
//!
//! NOTE: This implementation does NOT include encryption nor the optional
//! flow-control / out-of-band escape processing of historical rlogin.
//! Treat it as a thin TCP wrapper with a one-time startup record.
//!
//! SECURITY: rlogin is plaintext. Do not use on untrusted networks.
//! Consider SSH for secure remote execution.

use std::io::ErrorKind;
use std::time::Duration;

use async_trait::async_trait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::telnet::TerminalEmulation;

use super::{Connection, ConnectionState, ConnectionType};

/// Configuration for establishing an rlogin-style session.
///
/// Fields:
/// - `user_name` / `password`: Used in the repurposed rlogin handshake some
///   BBS systems expect. They may NOT correspond to classic rlogin semantics.
/// - `terminal_emulation`: Converted to a terminal capability string plus
///   a fixed baud token (e.g. "ANSI/115200") in `terminal_str()`.
/// - `swapped`: If true, reverses the field order in the handshake.
/// - `escape_sequence`: Optional raw byte slice which, if *exactly* written
///   via `send()`, causes a local shutdown. This is an application-level
///   convenience—not a protocol standard.
///
/// The terminal string is appended with a hard-coded "115200" fallback speed.
/// If a dynamic rate becomes necessary, extend this struct.
#[derive(Clone, Debug)]
pub struct RloginConfig {
    pub user_name: String,
    pub password: String,
    pub terminal_emulation: TerminalEmulation,
    pub swapped: bool,
    /// Optional escape sequence to trigger local disconnect (client side only).
    /// If set and an outgoing `send()` buffer matches exactly, we mark closed.
    pub escape_sequence: Option<Vec<u8>>,
}

impl RloginConfig {
    /// Build terminal capability string of the form "<EMULATION>/115200".
    /// Historically the field includes the terminal *and* speed; we fix the
    /// speed for simplicity. Extend if negotiation becomes necessary.
    pub fn terminal_str(&self) -> String {
        let terminal = match self.terminal_emulation {
            TerminalEmulation::Ansi | TerminalEmulation::Utf8Ansi => "ANSI",
            TerminalEmulation::Avatar => "AVATAR",
            TerminalEmulation::Rip => "RIP",           // RIPTerm (vector protocol)
            TerminalEmulation::Skypix => "SKYPIX",     // Skypix graphics/extended
            TerminalEmulation::Ascii => "VT100",       // Generic ASCII fallback
            TerminalEmulation::PETscii => "PETSCII",   // Commodore PET/64 family
            TerminalEmulation::ATAscii => "ATASCII",   // Atari 8-bit special set
            TerminalEmulation::ViewData => "VIEWDATA", // Prestel/Teletext style
            TerminalEmulation::Mode7 => "MODE7",       // BBC Micro Mode 7
            TerminalEmulation::AtariST => "ATARIST",   // Atari ST console flavor
        };
        format!("{terminal}/115200")
    }
}

/// Active RLogin connection wrapper.
/// Maintains a small internal read buffer to support non-blocking poll
/// semantics similar to other `Connection` implementations.
///
/// Lifecycle:
/// 1. `open()` performs TCP connect + handshake emission.
/// 2. Reads and writes are raw passthrough (no option negotiation).
/// 3. `poll()` attempts a non-blocking read to detect closure.
/// 4. `shutdown()` marks the connection closed.
///
/// This does not implement the historical "~." local escape sequence.
/// You can emulate this at a higher layer (line discipline).
pub struct RloginConnection {
    stream: TcpStream,
    cfg: RloginConfig,
    read_buffer: Vec<u8>,
    closed: bool,
}

impl RloginConnection {
    /// Open a new rlogin-style connection.
    ///
    /// Steps:
    /// - Append default port 513 if none specified.
    /// - Connect with timeout.
    /// - Set `TCP_NODELAY` (interactive responsiveness).
    /// - Construct handshake in the selected order:
    ///     Initial NUL
    ///     (password OR username depending on `swapped`) + NUL
    ///     (username OR password) + NUL
    ///     terminal string + NUL
    /// - Send handshake.
    ///
    /// Returns a ready `RloginConnection`. Does not validate remote response;
    /// classic rlogin has minimal startup acknowledgement.
    pub async fn open(addr: impl Into<String>, cfg: RloginConfig, timeout: Duration) -> crate::Result<Self> {
        let mut addr = addr.into();
        if !addr.contains(':') {
            addr.push_str(":513");
        }
        let mut stream = match tokio::time::timeout(timeout, TcpStream::connect(addr)).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(Box::new(e)),
            Err(e) => return Err(Box::new(e)),
        };
        stream.set_nodelay(true)?;

        let terminal = cfg.terminal_str();

        // Allocate handshake buffer once.
        // Layout (BBS variant):
        //   0 <first-field> 0 <second-field> 0 <terminal> 0
        // Where first/second depend on `swapped`.
        let mut handshake = Vec::with_capacity(1 + cfg.user_name.len() + 1 + cfg.password.len() + 1 + terminal.len() + 1);
        handshake.push(0);
        if cfg.swapped {
            // Reversed order: user then password
            handshake.extend_from_slice(cfg.user_name.as_bytes());
            handshake.push(0);
            handshake.extend_from_slice(cfg.password.as_bytes());
        } else {
            // Default BBS ordering: password then user
            handshake.extend_from_slice(cfg.password.as_bytes());
            handshake.push(0);
            handshake.extend_from_slice(cfg.user_name.as_bytes());
        }
        handshake.push(0);
        handshake.extend_from_slice(terminal.as_bytes());
        handshake.push(0);

        stream.write_all(&handshake).await?;

        Ok(Self {
            stream,
            cfg,
            read_buffer: Vec::new(),
            closed: false,
        })
    }

    /// Server-side wrapper accept. A real server would:
    /// - Read until four NUL-delimited fields are received.
    /// - Parse order (potentially auto-detect swapped variant).
    /// This implementation leaves parsing to future enhancements.
    pub async fn accept(stream: TcpStream, cfg: RloginConfig) -> crate::Result<Self> {
        Ok(Self {
            stream,
            cfg,
            read_buffer: Vec::new(),
            closed: false,
        })
    }

    /// Drain internal buffer into caller's destination slice.
    /// Returns number of bytes copied.
    fn buffer_drain_into(&mut self, dst: &mut [u8]) -> usize {
        if self.read_buffer.is_empty() {
            return 0;
        }
        let n = dst.len().min(self.read_buffer.len());
        dst[..n].copy_from_slice(&self.read_buffer[..n]);
        self.read_buffer.drain(..n);
        n
    }

    /// Attempt a non-blocking read to pre-fill `read_buffer`.
    /// Only one short read (512 bytes) is attempted to limit latency.
    /// On EOF or disconnection errors we mark `closed`.
    async fn nonblocking_fill(&mut self) -> crate::Result<()> {
        let mut tmp = [0u8; 512];
        match self.stream.try_read(&mut tmp) {
            Ok(0) => {
                self.closed = true;
            }
            Ok(n) => {
                self.read_buffer.extend_from_slice(&tmp[..n]);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) if is_disconnection_error(&e) => {
                self.closed = true;
            }
            Err(e) => return Err(Box::new(e)),
        }
        Ok(())
    }

    /// Check if outgoing payload matches configured escape sequence.
    /// If matched, we trigger a local shutdown. This is *not* protocol-level
    /// semantics—pure convenience for embedding a session abort.
    fn check_escape(&mut self, buf: &[u8]) -> bool {
        if let Some(seq) = &self.cfg.escape_sequence {
            if buf == seq {
                return true;
            }
        }
        false
    }
}

/// Helper: classify connection-ending IO errors.
fn is_disconnection_error(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset | ErrorKind::NotConnected | ErrorKind::BrokenPipe | ErrorKind::UnexpectedEof
    )
}

#[async_trait]
impl Connection for RloginConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Rlogin
    }

    /// Blocking read with internal buffering.
    /// Strategy:
    /// 1. Serve any previously buffered bytes.
    /// 2. If connection is flagged closed, return 0.
    /// 3. Perform a normal read; on 0 or error mark closed.
    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        let drained = self.buffer_drain_into(buf);
        if drained > 0 {
            return Ok(drained);
        }
        if self.closed {
            return Ok(0);
        }

        match self.stream.read(buf).await {
            Ok(0) => {
                self.closed = true;
                Ok(0)
            }
            Ok(n) => Ok(n),
            Err(e) if is_disconnection_error(&e) => {
                self.closed = true;
                Ok(0)
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Non-blocking read variant. Returns immediately:
    /// - Buffered data first.
    /// - Then tries a single `try_read`.
    /// - Maps disconnection errors to `closed`.
    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        let drained = self.buffer_drain_into(buf);
        if drained > 0 {
            return Ok(drained);
        }
        if self.closed {
            return Ok(0);
        }

        match self.stream.try_read(buf) {
            Ok(0) => {
                self.closed = true;
                Ok(0)
            }
            Ok(n) => Ok(n),
            Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(0),
            Err(e) if is_disconnection_error(&e) => {
                self.closed = true;
                Ok(0)
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Poll for connection state changes without consuming data.
    /// Performs a non-blocking fill to detect remote closure and prime read buffer.
    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        if self.closed {
            return Ok(ConnectionState::Disconnected);
        }
        self.nonblocking_fill().await?;
        Ok(if self.closed {
            ConnectionState::Disconnected
        } else {
            ConnectionState::Connected
        })
    }

    /// Write raw bytes. If buffer matches escape sequence, close locally.
    /// No character escaping is performed (unlike Telnet).
    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        if self.check_escape(buf) {
            self.closed = true;
            return Ok(());
        }
        self.stream.write_all(buf).await?;
        Ok(())
    }

    /// Graceful local shutdown. Attempts TCP half-close; marks state closed.
    async fn shutdown(&mut self) -> crate::Result<()> {
        let _ = self.stream.shutdown().await;
        self.closed = true;
        Ok(())
    }
}
