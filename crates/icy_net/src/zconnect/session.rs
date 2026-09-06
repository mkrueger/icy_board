//! Bounded, caller-side Chapter II ZCONNECT implementation.
//!
//! Profile: preconfigured authentication, public mail, ZIP and half-duplex
//! ZMODEM only. No automatic password enrollment, private mail, encryption,
//! file requests, deferred execution, or alternate transfer/mail formats.
//! Telnet supplies no confidentiality: login and header passwords are cleartext.
//! No protocol bodies or credentials are logged by this module.
//!
//! A successful file transfer is NOT a receipt. Only a subsequent `request`
//! completing BLK1/ACK1/TME1 and checking BLK2 for RETRANSMIT permits retirement
//! of the previous outbound packet. Downloads are synced before that exchange.

use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use tokio::time::{Instant, timeout, timeout_at};

use super::{
    BlockCode, EndTransmission, ProtocolTransition, ZConnectBlock, ZConnectState,
    commands::{Execute, ZConnectCmd, ZConnectCommandBlock},
    header::{Acer, Mailer, Mailformat, TransferProtocol, ZConnectHeaderBlock},
};
use crate::{Connection, ConnectionState, ConnectionType, Result, protocol::TransferProtocolType};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("ZCONNECT timeout")]
    Timeout,
    #[error("ZCONNECT peer disconnected")]
    Eof,
    #[error("ZCONNECT retry limit exceeded")]
    Retries,
    #[error("ZCONNECT invalid block")]
    InvalidBlock,
    #[error("ZCONNECT unexpected control/state")]
    Unexpected,
    #[error("ZCONNECT limit exceeded")]
    Limit,
    #[error("ZCONNECT peer requested retransmission; retain outbound packet")]
    Retransmit,
    #[error("ZCONNECT peer refused the request")]
    Refused,
    #[error("ZCONNECT unsupported profile: {0}")]
    Unsupported(&'static str),
}

#[derive(Clone)]
pub struct Limits {
    pub timeout: Duration,
    pub retries: usize,
    pub block_bytes: usize,
    pub packet_bytes: u64,
    pub transfer_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            retries: 15,
            block_bytes: 64 * 1024,
            packet_bytes: 64 * 1024 * 1024,
            transfer_timeout: Duration::from_secs(15 * 60),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Login {
    Direct,
    Zconnect,
    Janus,
}

/// Intentionally not Debug: contains a secret. `remote_system` is the expected
/// SYS display name (empty disables the name check), not the dial host and not
/// cryptographic peer authentication. The peer must still supply a nonempty SYS.
pub struct Identity<'a> {
    pub system: &'a str,
    pub sysop: &'a str,
    pub login_system: &'a str,
    pub remote_system: &'a str,
    pub password: &'a str,
}

/// Bytes read ahead while parsing text remain here for the binary protocol.
/// Neither command framing nor a handoff clears this queue.
struct Buffered<C> {
    inner: C,
    pending: VecDeque<u8>,
    receiving: bool,
    trailer: usize,
    transfer_prefix: bool,
}

#[async_trait]
impl<C: Connection> Connection for Buffered<C> {
    fn get_connection_type(&self) -> ConnectionType {
        self.inner.get_connection_type()
    }
    async fn read(&mut self, out: &mut [u8]) -> Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        while self.transfer_prefix {
            if self.pending.is_empty() {
                let mut byte = [0];
                if self.inner.read(&mut byte).await? == 0 {
                    return Err(SessionError::Eof.into());
                }
                self.pending.push_back(byte[0]);
            }
            // LF is ignored by Chapter II framing and may trail the last CR.
            // Stop before the first binary byte, including every ZPAD/ZDLE.
            if matches!(self.pending.front(), Some(b'\r' | b'\n')) {
                self.pending.pop_front();
            } else {
                self.transfer_prefix = false;
            }
        }
        if self.trailer != 0 {
            // The existing receiver probes for optional OO after ZFIN. Its
            // probe otherwise consumes the first byte of the next text block.
            // Peek here and keep non-trailer bytes for the command parser.
            if self.pending.is_empty() {
                let mut byte = [0];
                if self.inner.read(&mut byte).await? == 0 {
                    return Err(SessionError::Eof.into());
                }
                self.pending.push_back(byte[0]);
            }
            if self.pending.front() != Some(&b'O') {
                return Err(SessionError::Unexpected.into());
            }
            self.trailer -= 1;
            out[0] = self.pending.pop_front().unwrap();
            return Ok(1);
        }
        if !self.pending.is_empty() {
            let n = out.len().min(self.pending.len());
            for byte in &mut out[..n] {
                *byte = self.pending.pop_front().unwrap();
            }
            return Ok(n);
        }
        self.inner.read(out).await
    }
    async fn try_read(&mut self, out: &mut [u8]) -> Result<usize> {
        if self.transfer_prefix {
            if out.is_empty() {
                return Ok(0);
            }
            if self.pending.is_empty() {
                let mut buffer = [0; 4096];
                let n = self.inner.try_read(&mut buffer).await?;
                self.pending.extend(&buffer[..n]);
            }
            while matches!(self.pending.front(), Some(b'\r' | b'\n')) {
                self.pending.pop_front();
            }
            if self.pending.is_empty() {
                return Ok(0);
            }
            self.transfer_prefix = false;
        }
        if !self.pending.is_empty() {
            self.read(out).await
        } else {
            self.inner.try_read(out).await
        }
    }
    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        if self.receiving && bytes.starts_with(b"**\x18B08") {
            self.trailer = 2;
        }
        self.inner.send(bytes).await
    }
    async fn poll(&mut self) -> Result<ConnectionState> {
        self.inner.poll().await
    }
    async fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown().await
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Frame {
    state: ZConnectState,
    lines: Vec<String>,
}

impl ZConnectBlock for Frame {
    fn state(&self) -> ZConnectState {
        self.state
    }
    fn set_state(&mut self, state: ZConnectState) {
        self.state = state;
    }
    fn generate_lines(&self) -> Vec<String> {
        self.lines.clone()
    }
    fn parse_cmd(&mut self, command: &str, value: String) -> Result<()> {
        self.lines.push(format!("{command}:{value}"));
        Ok(())
    }
}

impl Frame {
    fn control(state: ZConnectState) -> Self {
        Self { state, lines: Vec::new() }
    }
    fn has(&self, key: &str) -> bool {
        self.lines.iter().any(|s| s.split_once(':').is_some_and(|(k, _)| k == key))
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    New,
    Ready,
    Execute,
    Transfer,
    Failed,
}

/// A checked BLK2/BLK4 response. Absent PUT supports legacy systems which use
/// TME4 rather than an empty PUT to indicate no mail.
#[derive(Default, Debug)]
pub struct Reply {
    pub offered: Option<u8>,
    pub bytes: Option<u64>,
    pub crc: Option<u32>,
    wait: u32,
    refused: bool,
    closing: bool,
}

impl Reply {
    fn merge(&mut self, frame: &Frame, limits: &Limits) -> Result<()> {
        if frame.has("RETRANSMIT") {
            return Err(SessionError::Retransmit.into());
        }
        if frame.has("LOGOFF") && !self.closing {
            return Err(SessionError::Refused.into());
        }
        // Mail flags must not be silently reduced by the legacy command parser.
        for line in &frame.lines {
            if let Some(value) = line.strip_prefix("PUT:")
                && !value.bytes().all(|b| b == b'B' || b == b'b')
            {
                return Err(SessionError::Unsupported("only public mail (B) is supported").into());
            }
        }
        let block = ZConnectCommandBlock::parse(&frame.display())?;
        for command in block.commands() {
            match command {
                ZConnectCmd::Put(flags) => {
                    if self.offered.is_some_and(|old| old != *flags) {
                        return Err(SessionError::Unexpected.into());
                    }
                    self.offered = Some(*flags);
                }
                ZConnectCmd::Execute(Execute::Yes) => {}
                ZConnectCmd::Execute(Execute::No) | ZConnectCmd::Wait(None) => self.refused = true,
                ZConnectCmd::Execute(Execute::Later) => return Err(SessionError::Unsupported("deferred execution").into()),
                ZConnectCmd::Wait(Some(seconds)) => {
                    if u64::from(*seconds) > limits.transfer_timeout.as_secs() {
                        return Err(SessionError::Limit.into());
                    }
                    self.wait = self.wait.max(*seconds);
                }
                ZConnectCmd::Bytes(n) => {
                    if *n > limits.packet_bytes || self.bytes.is_some_and(|old| old != *n) {
                        return Err(SessionError::Limit.into());
                    }
                    self.bytes = Some(*n);
                }
                ZConnectCmd::FileCrc(n) => {
                    if self.crc.is_some_and(|old| old != *n) {
                        return Err(SessionError::Unexpected.into());
                    }
                    self.crc = Some(*n);
                }
                ZConnectCmd::Format(s) if s.eq_ignore_ascii_case("ZCONNECT") => {}
                ZConnectCmd::Logoff if self.closing => {}
                _ => return Err(SessionError::Unsupported("response command or mail format").into()),
            }
        }
        Ok(())
    }
}

pub struct Session<C> {
    io: Buffered<C>,
    limits: Limits,
    partial: Vec<u8>,
    raw_count: usize,
    last_sent: Option<BlockCode>,
    last_received: Option<Frame>,
    phase: Phase,
}

impl<C: Connection> Session<C> {
    pub fn new(connection: C, limits: Limits) -> Result<Self> {
        if limits.timeout.is_zero()
            || limits.transfer_timeout.is_zero()
            || limits.retries > 15
            || !(128..=1024 * 1024).contains(&limits.block_bytes)
            || limits.packet_bytes == 0
            || limits.packet_bytes > u32::MAX as u64
        {
            return Err(SessionError::Limit.into());
        }
        Ok(Self {
            io: Buffered {
                inner: connection,
                pending: VecDeque::new(),
                receiving: false,
                trailer: 0,
                transfer_prefix: false,
            },
            limits,
            partial: Vec::new(),
            raw_count: 0,
            last_sent: None,
            last_received: None,
            phase: Phase::New,
        })
    }

    /// Tighten the per-packet bound to the caller's remaining session budget.
    /// Cannot raise the original limit or change it during a transfer.
    pub fn limit_packet_bytes(&mut self, maximum: u64) -> Result<()> {
        if maximum == 0 || self.phase == Phase::Transfer {
            return Err(SessionError::Limit.into());
        }
        self.limits.packet_bytes = self.limits.packet_bytes.min(maximum);
        Ok(())
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        timeout(self.limits.timeout, self.io.send(bytes)).await.map_err(|_| SessionError::Timeout)?
    }

    async fn control(&mut self, state: ZConnectState) -> Result<()> {
        self.send(Frame::control(state).display().as_bytes()).await
    }

    async fn byte(&mut self) -> Result<u8> {
        if self.io.pending.is_empty() {
            let mut buf = [0u8; 4096];
            let n = self.io.inner.read(&mut buf).await?;
            if n == 0 {
                return Err(SessionError::Eof.into());
            }
            self.io.pending.extend(&buf[..n]);
        }
        Ok(self.io.pending.pop_front().unwrap())
    }

    // Partial frames survive cancellation by the outer timeout. Only the frame
    // terminator is consumed; any following command/ZMODEM bytes remain queued.
    async fn read_frame(&mut self) -> Result<Frame> {
        loop {
            let b = self.byte().await?;
            self.raw_count += 1;
            if self.raw_count > self.limits.block_bytes * 2 {
                return Err(SessionError::Limit.into());
            }
            if b != b'\r' && !(b' '..=b'~').contains(&b) {
                continue;
            }
            if self.partial.is_empty() && b == b'\r' {
                continue;
            }
            self.partial.push(b);
            // BEGIN is repeated during login; it is not a protocol block.
            if self.partial.eq_ignore_ascii_case(b"BEGIN\r") {
                self.partial.clear();
                continue;
            }
            if self.partial.len() > self.limits.block_bytes {
                return Err(SessionError::Limit.into());
            }
            if self.partial.ends_with(b"\r\r") {
                let bytes = std::mem::take(&mut self.partial);
                self.raw_count = 0;
                let mut frame = Frame::default();
                frame.parse_block(std::str::from_utf8(&bytes)?).map_err(|_| SessionError::InvalidBlock)?;
                if !matches!(frame.state, ZConnectState::Block(_)) && !frame.lines.is_empty() {
                    return Err(SessionError::InvalidBlock.into());
                }
                return Ok(frame);
            }
        }
    }

    async fn next_frame(&mut self) -> Result<Frame> {
        for _ in 0..=self.limits.retries {
            let duration = self.limits.timeout;
            let frame = timeout(duration, self.read_frame()).await.map_err(|_| SessionError::Timeout)??;
            if let ZConnectState::Ack(n) = frame.state
                && self.last_sent == Some(n)
            {
                self.control(ZConnectState::Tme(n)).await?;
                continue;
            }
            if let Some(previous) = &self.last_received {
                if frame.state == previous.state {
                    if &frame != previous {
                        return Err(SessionError::Unexpected.into());
                    }
                    if let ZConnectState::Block(n) = frame.state {
                        self.control(ZConnectState::Ack(n)).await?;
                        continue;
                    }
                }
                if let ZConnectState::Block(n) = previous.state
                    && frame.state == ZConnectState::Tme(n)
                {
                    continue;
                }
            }
            return Ok(frame);
        }
        Err(SessionError::Retries.into())
    }

    fn retryable(error: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
        matches!(error.downcast_ref::<SessionError>(), Some(SessionError::Timeout | SessionError::InvalidBlock))
    }

    async fn send_block(&mut self, n: BlockCode, block: &dyn ZConnectBlock) -> Result<()> {
        let frame = Frame {
            state: ZConnectState::Block(n),
            lines: block.generate_lines(),
        };
        let wire = frame.display(); // cache once, including randomized map order
        if wire.len() > self.limits.block_bytes {
            return Err(SessionError::Limit.into());
        }
        for _ in 0..=self.limits.retries {
            self.send(wire.as_bytes()).await?;
            match self.next_frame().await {
                Ok(reply) if reply.state == ZConnectState::Ack(n) => {
                    self.control(ZConnectState::Tme(n)).await?;
                    self.last_sent = Some(n);
                    return Ok(());
                }
                Ok(reply) if reply.state == ZConnectState::Nak0 => {}
                Ok(_) => return Err(SessionError::Unexpected.into()),
                Err(e) if Self::retryable(e.as_ref()) => {}
                Err(e) => return Err(e),
            }
        }
        Err(SessionError::Retries.into())
    }

    /// Returns whether BLK4 ended with EOT4 (external transfer), not TME4.
    async fn receive_block(&mut self, n: BlockCode) -> Result<(Frame, bool)> {
        let mut received = None;
        for _ in 0..=self.limits.retries {
            match self.next_frame().await {
                Ok(frame) if frame.state == ZConnectState::Block(n) => {
                    received = Some(frame);
                    break;
                }
                Ok(_) => return Err(SessionError::Unexpected.into()),
                Err(e) if Self::retryable(e.as_ref()) => self.control(ZConnectState::Nak0).await?,
                Err(e) => return Err(e),
            }
        }
        let frame = received.ok_or(SessionError::Retries)?;
        self.control(ZConnectState::Ack(n)).await?;
        let mut eots = 0;
        for _ in 0..=self.limits.retries + 3 {
            match self.next_frame().await {
                Ok(next) if next.state == ZConnectState::Tme(n) && eots == 0 => {
                    self.last_received = Some(frame.clone());
                    return Ok((frame, false));
                }
                Ok(next) if n == BlockCode::Block4 && next.state == ZConnectState::Eot(EndTransmission::End4) => {
                    eots += 1;
                    if eots == 3 {
                        self.last_received = Some(frame.clone());
                        return Ok((frame, true));
                    }
                }
                Ok(next) if next == frame || next.state == ZConnectState::Nak0 => {
                    eots = 0;
                    self.control(ZConnectState::Ack(n)).await?;
                }
                Ok(_) => return Err(SessionError::Unexpected.into()),
                Err(e) if Self::retryable(e.as_ref()) => self.control(ZConnectState::Ack(n)).await?,
                Err(e) => return Err(e),
            }
        }
        Err(SessionError::Retries.into())
    }

    /// Standard login uses the public zconnect/0zconnec dispatcher, followed by
    /// authenticated headers. Janus uses JANUS, local SYS, and link password.
    pub async fn login(&mut self, login: Login, identity: &Identity<'_>) -> Result<()> {
        if self.phase != Phase::New {
            return Err(SessionError::Unexpected.into());
        }
        validate_expected_system(identity.remote_system)?;
        for field in [identity.system, identity.sysop, identity.login_system, identity.password] {
            if field.is_empty() || field.len() > 255 || !field.bytes().all(|b| (b' '..=b'~').contains(&b)) {
                return Err(SessionError::Unsupported("non-ASCII/empty identity or control characters").into());
            }
        }
        if identity.password.len() > 10 {
            return Err(SessionError::Unsupported("Chapter II password exceeds 10 characters").into());
        }
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut text = Vec::new();
        let mut total = 0usize;
        let mut attempts = 0;
        loop {
            let byte = timeout_at(deadline.min(Instant::now() + Duration::from_secs(10)), self.byte()).await;
            match byte {
                Err(_) => {
                    attempts += 1;
                    if attempts > 12 || Instant::now() >= deadline {
                        return Err(SessionError::Timeout.into());
                    }
                    if !matches!(login, Login::Direct) {
                        self.send(b"\r").await?;
                    }
                    continue;
                }
                Ok(Err(e)) => return Err(e),
                Ok(Ok(b)) => {
                    total += 1;
                    if total > self.limits.block_bytes {
                        return Err(SessionError::Limit.into());
                    }
                    text.push(b.to_ascii_lowercase());
                    if text.len() > 128 {
                        text.remove(0);
                    }
                }
            }
            if text.ends_with(b"begin\r") {
                return Ok(());
            }
            let response: Option<&str> = match login {
                Login::Direct => None,
                Login::Zconnect if text.ends_with(b"ogin") || text.ends_with(b"ame") => Some("zconnect"),
                Login::Janus if text.ends_with(b"username:") => Some("JANUS"),
                Login::Janus if text.ends_with(b"systemname:") => Some(identity.login_system),
                Login::Zconnect if text.ends_with(b"word") || text.ends_with(b"wort") => Some("0zconnec"),
                Login::Janus if text.ends_with(b"word") || text.ends_with(b"wort") => Some(identity.password),
                _ => None,
            };
            if let Some(response) = response {
                attempts += 1;
                if attempts > 15 {
                    return Err(SessionError::Retries.into());
                }
                // The standard asks for a one-second pause after the login banner.
                // Do not read/discard its tail: it remains available to the next step.
                if matches!(login, Login::Zconnect) && response == "zconnect" {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                self.send(format!("{response}\r").as_bytes()).await?;
                text.clear();
            }
        }
    }

    pub async fn negotiate(&mut self, identity: &Identity<'_>) -> Result<()> {
        if self.phase != Phase::New {
            return Err(SessionError::Unexpected.into());
        }
        // This entry point is public too: do not rely on login() having already
        // checked the strings before serializing the authentication header.
        validate_expected_system(identity.remote_system)?;
        for field in [identity.system, identity.sysop, identity.password] {
            if field.is_empty() || field.len() > 255 || !field.bytes().all(|b| (b' '..=b'~').contains(&b)) {
                return Err(SessionError::Unsupported("invalid header identity").into());
            }
        }
        if identity.password.len() > 10 {
            return Err(SessionError::Unsupported("Chapter II password exceeds 10 characters").into());
        }
        self.phase = Phase::Failed;
        let mut header = ZConnectHeaderBlock::default();
        header.set_system(identity.system);
        header.set_sysop(identity.sysop);
        header.set_password(identity.password);
        header.add_phone(0, "TCP");
        header.add_acer(0, Acer::ZIP2);
        header.add_acer(0, Acer::ZIP);
        header.add_protocol(0, TransferProtocol::ZModem);
        header.add_mailer(0, Mailer::ZConnect);
        header.add_mailformat(0, Mailformat::ZConnect);
        self.send_block(BlockCode::Block1, &header).await?;
        let (remote, transfer) = self.receive_block(BlockCode::Block2).await?;
        if transfer || remote.has("LOGOFF") {
            return Err(SessionError::Refused.into());
        }
        if remote.lines.iter().filter(|line| line.starts_with("SYS:")).count() != 1 {
            return Err(SessionError::InvalidBlock.into());
        }
        for key in ["SYSOP:", "PORT:"] {
            if remote.lines.iter().filter(|line| line.starts_with(key)).count() != 1 {
                return Err(SessionError::InvalidBlock.into());
            }
        }
        if !remote.has("TEL") || remote.lines.iter().filter(|line| line.starts_with("PASSWD:")).count() > 1 {
            return Err(SessionError::InvalidBlock.into());
        }
        // Parse the actual system header, NEVER a command block.
        let remote_header = ZConnectHeaderBlock::parse(&remote.display())?;
        validate_header(&remote_header, identity)?;
        let archive = if remote_header.acer(remote_header.port()).is_some_and(|v| v.contains(&Acer::ZIP2)) {
            "ZIP2"
        } else {
            "ZIP"
        };
        let selection = Frame {
            state: ZConnectState::default(),
            lines: vec!["Proto:ZMODEM".into(), format!("ArcerIn:{archive}"), format!("ArcerOut:{archive}")],
        };
        self.send_block(BlockCode::Block3, &selection).await?;
        let (final_header, transfer) = self.receive_block(BlockCode::Block4).await?;
        if transfer || final_header.has("LOGOFF") {
            return Err(SessionError::Refused.into());
        }
        // BLK4 is a negotiation confirmation, not a command block. Empty is legal.
        for line in &final_header.lines {
            let (key, value) = line.split_once(':').ok_or(SessionError::InvalidBlock)?;
            match key {
                "PROTO" if value.eq_ignore_ascii_case("ZMODEM") => {}
                "ARCERIN" | "ARCEROUT" | "ACERIN" | "ACEROUT" if value.eq_ignore_ascii_case(archive) => {}
                _ => return Err(SessionError::Unsupported("negotiation confirmation").into()),
            }
        }
        self.phase = Phase::Ready;
        Ok(())
    }

    /// Complete the next receipt boundary, checking the following BLK2 for a
    /// RETRANSMIT override before reporting success. Callers may now acknowledge
    /// the PREVIOUS upload; this does not acknowledge the request being started.
    pub async fn request(&mut self, command: &ZConnectCommandBlock) -> Result<Reply> {
        if self.phase != Phase::Ready {
            return Err(SessionError::Unexpected.into());
        }
        let mut transfers = 0;
        for cmd in command.commands() {
            match cmd {
                ZConnectCmd::Get(flags) | ZConnectCmd::Put(flags) if *flags == super::commands::mails::NEWS => transfers += 1,
                ZConnectCmd::Delete(flags) if *flags == super::commands::mails::NEWS => {}
                ZConnectCmd::Format(value) if value.eq_ignore_ascii_case("ZCONNECT") => {}
                ZConnectCmd::Bytes(n) if *n <= self.limits.packet_bytes => {}
                ZConnectCmd::Logoff | ZConnectCmd::FileCrc(_) => {}
                _ => return Err(SessionError::Unsupported("outgoing command; only public-mail GET/PUT/DELETE and LOGOFF").into()),
            }
        }
        if transfers > 1 {
            return Err(SessionError::Unsupported("combined or batch transfer").into());
        }
        self.phase = Phase::Failed;
        self.send_block(BlockCode::Block1, command).await?;
        let (frame, _) = self.receive_block(BlockCode::Block2).await?;
        let mut reply = Reply {
            closing: command.commands().iter().any(|c| matches!(c, ZConnectCmd::Logoff)),
            ..Reply::default()
        };
        reply.merge(&frame, &self.limits)?;
        self.phase = Phase::Execute;
        Ok(reply)
    }

    /// Complete BLK3/4 and optionally the WAIT/BEG5/EOT5 packing handshake.
    /// Returns false on a TME4/no-data cycle. Positive WAIT is bounded by the
    /// transfer deadline; caller-side packing/deferred work is not advertised.
    pub async fn execute(&mut self, reply: &mut Reply, accept: bool) -> Result<bool> {
        if self.phase != Phase::Execute {
            return Err(SessionError::Unexpected.into());
        }
        self.phase = Phase::Failed;
        let accept = accept && !reply.refused && reply.offered != Some(0);
        self.send_block(
            BlockCode::Block3,
            &ZConnectCommandBlock::default().execute(if accept { Execute::Yes } else { Execute::No }),
        )
        .await?;
        let (frame, transfer) = self.receive_block(BlockCode::Block4).await?;
        reply.merge(&frame, &self.limits)?;
        if transfer && (!accept || reply.refused) {
            return Err(SessionError::Unexpected.into());
        }
        if transfer && reply.wait > 0 {
            let deadline = Instant::now() + self.limits.transfer_timeout;
            let mut ready = false;
            for _ in 0..=self.limits.retries {
                match timeout_at(deadline, self.next_frame()).await.map_err(|_| SessionError::Timeout)? {
                    Ok(frame) if frame.state == ZConnectState::Begin(ProtocolTransition::Prot5) => {
                        ready = true;
                        break;
                    }
                    Ok(_) => return Err(SessionError::Unexpected.into()),
                    Err(e) if Self::retryable(e.as_ref()) => {}
                    Err(e) => return Err(e),
                }
            }
            if !ready {
                return Err(SessionError::Retries.into());
            }
            for n in 0..3 {
                if n != 0 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                self.control(ZConnectState::Eot(EndTransmission::Prot5)).await?;
            }
        }
        self.phase = if transfer { Phase::Transfer } else { Phase::Ready };
        Ok(transfer)
    }

    /// Send exactly one prebuilt archive. Success here is only transport success;
    /// retain it until the next successful `request` receipt boundary.
    pub async fn send_zip(&mut self, path: &Path) -> Result<()> {
        if self.phase != Phase::Transfer {
            return Err(SessionError::Unexpected.into());
        }
        self.phase = Phase::Failed;
        validate_zip(path, None, None, self.limits.packet_bytes)?;
        self.io.transfer_prefix = true;
        let mut protocol = TransferProtocolType::ZModem.create();
        let deadline = Instant::now() + self.limits.transfer_timeout;
        let mut state = timeout(self.limits.timeout, protocol.initiate_send(&mut self.io, &[path.to_path_buf()]))
            .await
            .map_err(|_| SessionError::Timeout)??;
        let result: Result<()> = async {
            while !state.is_finished {
                if Instant::now() >= deadline {
                    return Err(SessionError::Timeout.into());
                }
                timeout_at(deadline, protocol.update_transfer(&mut self.io, &mut state))
                    .await
                    .map_err(|_| SessionError::Timeout)??;
                if state.send_state.errors > 0 || state.request_cancel {
                    return Err(SessionError::Refused.into());
                }
                state.send_state.output_log.clear();
                tokio::task::yield_now().await;
            }
            if state.send_state.finished_files.len() != 1 {
                return Err(SessionError::Refused.into());
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            let _ = timeout(self.limits.timeout, protocol.cancel_transfer(&mut self.io)).await;
        }
        result?;
        self.phase = Phase::Ready;
        Ok(())
    }

    /// Receive one archive into an atomic, durable, collision-free local file.
    /// A completed archive is persisted immediately at ZEOF, so even a later
    /// ZFIN/connection failure leaves it in `inbound`. Never use a peer path.
    pub async fn receive_zip(&mut self, inbound: &Path, reply: &Reply) -> Result<PathBuf> {
        if self.phase != Phase::Transfer {
            return Err(SessionError::Unexpected.into());
        }
        self.phase = Phase::Failed;
        fs::create_dir_all(inbound)?;
        self.io.receiving = true;
        self.io.transfer_prefix = true;
        let mut protocol = TransferProtocolType::ZModem.create();
        let deadline = Instant::now() + self.limits.transfer_timeout;
        let mut state = timeout(self.limits.timeout, protocol.initiate_recv(&mut self.io))
            .await
            .map_err(|_| SessionError::Timeout)??;
        let mut durable = None;
        let mut advertised = None;
        let result: Result<()> = async {
            while !state.is_finished {
                if Instant::now() >= deadline {
                    return Err(SessionError::Timeout.into());
                }
                let update = timeout_at(deadline, protocol.update_transfer(&mut self.io, &mut state))
                    .await
                    .map_err(|_| SessionError::Timeout)?;
                let received = &mut state.recieve_state;
                if received.file_size > self.limits.packet_bytes
                    || received.total_bytes_transfered > self.limits.packet_bytes
                    || received.cur_bytes_transfered > self.limits.packet_bytes
                {
                    return Err(SessionError::Limit.into());
                }
                if !received.file_name.is_empty() {
                    safe_zip_name(&received.file_name)?;
                    if durable.is_some() {
                        return Err(SessionError::Unsupported("ZCONNECT packets are not batch transfers").into());
                    }
                    // ZMODEM permits an omitted size (reported as zero by the
                    // existing engine). Actual bytes remain independently bounded.
                    advertised = (received.file_size != 0).then_some(received.file_size);
                }
                for (name, temporary) in std::mem::take(&mut received.finished_files) {
                    let saved = (|| -> Result<PathBuf> {
                        safe_zip_name(&name)?;
                        if durable.is_some() {
                            return Err(SessionError::Unsupported("multiple archives").into());
                        }
                        validate_zip(&temporary, advertised, reply.crc, self.limits.packet_bytes)?;
                        validate_zip(&temporary, reply.bytes, None, self.limits.packet_bytes)?;
                        durable_zip(&temporary, inbound)
                    })();
                    let _ = fs::remove_file(&temporary);
                    durable = Some(saved?);
                }
                update?;
                // The receiver counts recoverable header errors in `errors`
                // too. Let the existing engine retry those; only fatal log
                // entries/cancellation or our independent bound abort here.
                if received.errors > 32
                    || received
                        .output_log
                        .iter()
                        .any(|entry| matches!(entry, crate::protocol::OutputLogMessage::Error(_)))
                    || state.request_cancel
                {
                    return Err(SessionError::Refused.into());
                }
                // Transfer logs contain peer strings and can grow unbounded.
                received.output_log.clear();
                tokio::task::yield_now().await;
            }
            if durable.is_none() {
                return Err(SessionError::Refused.into());
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            let _ = timeout(self.limits.timeout, protocol.cancel_transfer(&mut self.io)).await;
        }
        // Any unprocessed engine temporaries are ours to retire, not mail packets.
        for (_, path) in &state.recieve_state.finished_files {
            let _ = fs::remove_file(path);
        }
        self.io.receiving = false;
        self.io.trailer = 0;
        result?;
        self.phase = Phase::Ready;
        durable.ok_or_else(|| SessionError::Refused.into())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.phase = Phase::Failed;
        timeout(self.limits.timeout, self.io.shutdown()).await.map_err(|_| SessionError::Timeout)?
    }
}

fn validate_expected_system(system: &str) -> Result<()> {
    if system.len() > 255 || !system.bytes().all(|b| (b' '..=b'~').contains(&b)) {
        return Err(SessionError::Unsupported("invalid expected remote SYS name").into());
    }
    Ok(())
}

fn validate_header(header: &ZConnectHeaderBlock, identity: &Identity<'_>) -> Result<()> {
    if header.system().is_empty() {
        return Err(SessionError::InvalidBlock.into());
    }
    if !identity.remote_system.is_empty() && !header.system().eq_ignore_ascii_case(identity.remote_system) {
        return Err(SessionError::Unsupported("remote SYS does not match configured expected system name").into());
    }
    // Many peers send PASSWD only in BLK1. If supplied by the callee it must match.
    if !header.password().is_empty() && header.password() != identity.password {
        return Err(SessionError::Refused.into());
    }
    let port = header.port();
    if !header.acer(port).is_some_and(|a| a.contains(&Acer::ZIP) || a.contains(&Acer::ZIP2)) {
        return Err(SessionError::Unsupported("peer must support ZIP/ZIP2").into());
    }
    if !header.protocols(port).is_some_and(|p| p.contains(&TransferProtocol::ZModem)) {
        return Err(SessionError::Unsupported("peer must support ZMODEM").into());
    }
    if header.crypt(port).is_some() {
        return Err(SessionError::Unsupported("encrypted mail").into());
    }
    if header
        .mailer(port)
        .is_some_and(|v| !v.iter().any(|m| matches!(m, Mailer::ZConnect | Mailer::ZConnect3 | Mailer::ZConnect31)))
        || header.mailformat(port).is_some_and(|v| {
            !v.iter()
                .any(|m| matches!(m, Mailformat::ZConnect | Mailformat::ZConnect3 | Mailformat::ZConnect31))
        })
    {
        return Err(SessionError::Unsupported("non-ZCONNECT mail format").into());
    }
    Ok(())
}

fn safe_zip_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 128
        || name.starts_with('.')
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        || !(name.to_ascii_lowercase().ends_with(".zip") || name.to_ascii_lowercase().ends_with(".zi2"))
    {
        return Err(SessionError::Unsupported("unsafe inbound name or non-ZIP archive").into());
    }
    Ok(())
}

fn validate_zip(path: &Path, expected: Option<u64>, crc: Option<u32>, maximum: u64) -> Result<()> {
    let mut file = File::open(path)?;
    let meta = file.metadata()?;
    let size = meta.len();
    if !meta.is_file() || size > maximum || expected.is_some_and(|n| n != size) {
        return Err(SessionError::Limit.into());
    }
    let mut magic = [0; 4];
    file.read_exact(&mut magic)?;
    if magic != *b"PK\x03\x04" && magic != *b"PK\x05\x06" {
        return Err(SessionError::Unsupported("not a ZIP archive").into());
    }
    if let Some(expected_crc) = crc {
        let mut sum = u32::MAX;
        for byte in magic {
            sum = crate::crc::update_crc32(sum, byte);
        }
        let mut buffer = [0; 8192];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            for byte in &buffer[..n] {
                sum = crate::crc::update_crc32(sum, *byte);
            }
        }
        if !sum != expected_crc {
            return Err(SessionError::InvalidBlock.into());
        }
    }
    Ok(())
}

fn durable_zip(source: &Path, inbound: &Path) -> Result<PathBuf> {
    let mut stage = tempfile::Builder::new().prefix(".zconnect-").tempfile_in(inbound)?;
    let mut input = File::open(source)?;
    std::io::copy(&mut input, stage.as_file_mut())?;
    stage.flush()?;
    stage.as_file().sync_all()?;
    let name = stage.path().file_name().ok_or(SessionError::Unexpected)?.to_string_lossy();
    let destination = inbound.join(format!("{}.zip", name.trim_start_matches('.')));
    stage.persist_noclobber(&destination).map_err(|e| e.error)?;
    // Also sync directory ancestors: create_dir_all may have created the link
    // directory during this call, and its name must survive a crash as well.
    #[cfg(unix)]
    {
        let absolute = fs::canonicalize(inbound)?;
        for directory in absolute.ancestors() {
            File::open(directory)?.sync_all()?;
        }
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zconnect::commands::mails;
    use std::sync::{Arc, Mutex};

    enum Input {
        Bytes(VecDeque<u8>),
        Stall,
    }
    struct Peer {
        input: VecDeque<Input>,
        sent: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    #[async_trait]
    impl Connection for Peer {
        fn get_connection_type(&self) -> ConnectionType {
            ConnectionType::Channel
        }
        async fn read(&mut self, out: &mut [u8]) -> Result<usize> {
            if out.is_empty() {
                return Ok(0);
            }
            match self.input.pop_front() {
                None => Ok(0),
                Some(Input::Stall) => std::future::pending().await,
                Some(Input::Bytes(mut bytes)) => {
                    let n = out.len().min(bytes.len());
                    for byte in &mut out[..n] {
                        *byte = bytes.pop_front().unwrap();
                    }
                    if !bytes.is_empty() {
                        self.input.push_front(Input::Bytes(bytes));
                    }
                    Ok(n)
                }
            }
        }
        async fn try_read(&mut self, out: &mut [u8]) -> Result<usize> {
            self.read(out).await
        }
        async fn send(&mut self, bytes: &[u8]) -> Result<()> {
            self.sent.lock().unwrap().push(bytes.to_vec());
            Ok(())
        }
    }

    fn limits() -> Limits {
        Limits {
            timeout: Duration::from_millis(20),
            retries: 2,
            transfer_timeout: Duration::from_secs(10),
            ..Limits::default()
        }
    }

    fn scripted(bytes: Vec<u8>) -> Session<Peer> {
        Session::new(
            Peer {
                input: VecDeque::from([Input::Bytes(bytes.into())]),
                sent: Arc::default(),
            },
            limits(),
        )
        .unwrap()
    }

    fn wire(state: ZConnectState, lines: &[&str]) -> Vec<u8> {
        Frame {
            state,
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
        .display()
        .into_bytes()
    }

    fn sent(session: &Session<Peer>, state: ZConnectState) -> usize {
        session
            .io
            .inner
            .sent
            .lock()
            .unwrap()
            .iter()
            .filter(|bytes| String::from_utf8_lossy(bytes).contains(&format!("Status:{state}\r")))
            .count()
    }

    fn identity() -> Identity<'static> {
        Identity {
            system: "local.example",
            sysop: "sysop",
            login_system: "point",
            remote_system: "peer.example",
            password: "secret",
        }
    }

    #[tokio::test]
    async fn fragmented_crlf_and_coalesced_binary_tail_are_preserved() {
        let first = wire(ZConnectState::Ack(BlockCode::Block1), &[]);
        let second = wire(ZConnectState::Block(BlockCode::Block2), &["Put:B"]);
        let marker = b"**\x18B00000000000000\r\n\x11";
        let mut bytes = first;
        bytes.extend(second);
        let bytes = bytes
            .iter()
            .flat_map(|b| if *b == b'\r' { vec![b'\r', b'\n'] } else { vec![*b] })
            .collect::<Vec<_>>();
        let mut session = scripted(Vec::new());
        session.io.inner.input = bytes.into_iter().map(|b| Input::Bytes(VecDeque::from([b]))).collect();
        session.io.inner.input.push_back(Input::Bytes(marker.to_vec().into()));
        assert_eq!(session.read_frame().await.unwrap().state, ZConnectState::Ack(BlockCode::Block1));
        assert_eq!(session.read_frame().await.unwrap().state, ZConnectState::Block(BlockCode::Block2));
        let mut tail = vec![0; marker.len() + 1];
        // Last LF is ignored by text framing, but legitimately remains unread.
        session.io.read_exact(&mut tail).await.unwrap();
        assert_eq!(&tail[1..], marker);
    }

    #[tokio::test]
    async fn coalesced_frames_leave_exact_binary_marker() {
        let marker = b"**\x18B0100000023be50\r\n\x11";
        let mut bytes = wire(ZConnectState::Ack(BlockCode::Block1), &[]);
        bytes.extend(wire(ZConnectState::Tme(BlockCode::Block2), &[]));
        bytes.extend(marker);
        let mut session = scripted(bytes);
        session.read_frame().await.unwrap();
        session.read_frame().await.unwrap();
        let mut tail = vec![0; marker.len()];
        session.io.read_exact(&mut tail).await.unwrap();
        assert_eq!(tail, marker);
    }

    #[tokio::test]
    async fn missing_oo_does_not_eat_next_command() {
        let bytes = wire(ZConnectState::Nak0, &[]);
        let mut session = scripted(bytes);
        session.io.receiving = true;
        session.io.send(b"**\x18B0800000000022d\r\n").await.unwrap();
        assert!(session.io.read_u8().await.is_err());
        session.io.trailer = 0;
        assert_eq!(session.read_frame().await.unwrap().state, ZConnectState::Nak0);
    }

    #[tokio::test]
    async fn nak_retransmits_identical_block_and_checks_ack_number() {
        let mut bytes = wire(ZConnectState::Nak0, &[]);
        bytes.extend(wire(ZConnectState::Ack(BlockCode::Block1), &[]));
        let mut session = scripted(bytes);
        session
            .send_block(BlockCode::Block1, &ZConnectCommandBlock::default().get(mails::NEWS))
            .await
            .unwrap();
        let output = session.io.inner.sent.lock().unwrap();
        assert_eq!(output[0], output[1]);
        assert_eq!(output.len(), 3);
        drop(output);
        let mut session = scripted(wire(ZConnectState::Ack(BlockCode::Block2), &[]));
        assert!(session.send_block(BlockCode::Block1, &ZConnectCommandBlock::default()).await.is_err());
        assert_eq!(sent(&session, ZConnectState::Tme(BlockCode::Block1)), 0);
    }

    #[tokio::test]
    async fn timeout_preserves_partial_ack_and_resends() {
        let ack = wire(ZConnectState::Ack(BlockCode::Block1), &[]);
        let mut session = scripted(Vec::new());
        session.io.inner.input = VecDeque::from([Input::Bytes(ack[..8].to_vec().into()), Input::Stall, Input::Bytes(ack[8..].to_vec().into())]);
        session.send_block(BlockCode::Block1, &ZConnectCommandBlock::default()).await.unwrap();
        assert_eq!(sent(&session, ZConnectState::Block(BlockCode::Block1)), 2);
    }

    #[tokio::test]
    async fn eof_and_retry_exhaustion_are_bounded() {
        let mut session = scripted(b"Status:ACK".to_vec());
        let error = session.send_block(BlockCode::Block1, &ZConnectCommandBlock::default()).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<SessionError>(), Some(SessionError::Eof)));
        let mut session = scripted(wire(ZConnectState::Nak0, &[]).repeat(10));
        assert!(matches!(
            session
                .send_block(BlockCode::Block1, &ZConnectCommandBlock::default())
                .await
                .unwrap_err()
                .downcast_ref::<SessionError>(),
            Some(SessionError::Retries)
        ));
        assert_eq!(sent(&session, ZConnectState::Block(BlockCode::Block1)), 3);
    }

    #[tokio::test]
    async fn duplicate_ack_and_block_are_acknowledged_without_redelivery() {
        let block = wire(ZConnectState::Block(BlockCode::Block2), &["Put:B"]);
        let mut bytes = wire(ZConnectState::Ack(BlockCode::Block1), &[]);
        bytes.extend(wire(ZConnectState::Ack(BlockCode::Block1), &[]));
        bytes.extend(&block);
        bytes.extend(&block);
        bytes.extend(wire(ZConnectState::Tme(BlockCode::Block2), &[]));
        let mut session = scripted(bytes);
        session.send_block(BlockCode::Block1, &ZConnectCommandBlock::default()).await.unwrap();
        let (received, transfer) = session.receive_block(BlockCode::Block2).await.unwrap();
        assert!(!transfer);
        assert_eq!(received.lines, ["PUT:B"]);
        assert_eq!(sent(&session, ZConnectState::Tme(BlockCode::Block1)), 2);
        assert_eq!(sent(&session, ZConnectState::Ack(BlockCode::Block2)), 2);
    }

    #[tokio::test]
    async fn corrupt_crc_gets_nak_and_wrong_tme_is_not_success() {
        let block = wire(ZConnectState::Block(BlockCode::Block2), &["Put:B"]);
        let mut damaged = block.clone();
        damaged[0] ^= 1;
        damaged.extend(&block);
        damaged.extend(wire(ZConnectState::Tme(BlockCode::Block3), &[]));
        let mut session = scripted(damaged);
        assert!(session.receive_block(BlockCode::Block2).await.is_err());
        assert_eq!(sent(&session, ZConnectState::Nak0), 1);
    }

    #[tokio::test]
    async fn eot_requires_exactly_three_and_keeps_transfer_bytes() {
        let mut bytes = wire(ZConnectState::Block(BlockCode::Block4), &["Execute:J"]);
        bytes.extend(wire(ZConnectState::Eot(EndTransmission::End4), &[]).repeat(3));
        bytes.extend(b"**\x18B00");
        let mut session = scripted(bytes);
        assert!(session.receive_block(BlockCode::Block4).await.unwrap().1);
        assert_eq!(session.io.read_u8().await.unwrap(), b'*');
        let mut bytes = wire(ZConnectState::Block(BlockCode::Block4), &[]);
        bytes.extend(wire(ZConnectState::Eot(EndTransmission::End4), &[]).repeat(2));
        let mut session = scripted(bytes);
        assert!(session.receive_block(BlockCode::Block4).await.is_err());
    }

    #[tokio::test]
    async fn packing_handshake_checks_beg_number_and_preserves_zmodem_marker() {
        let mut bytes = wire(ZConnectState::Ack(BlockCode::Block3), &[]);
        bytes.extend(wire(ZConnectState::Block(BlockCode::Block4), &["Execute:J"]));
        bytes.extend(wire(ZConnectState::Eot(EndTransmission::End4), &[]).repeat(3));
        bytes.extend(wire(ZConnectState::Begin(ProtocolTransition::Prot5), &[]));
        bytes.extend(b"**\x18B00");
        let mut session = scripted(bytes);
        session.phase = Phase::Execute;
        let mut reply = Reply { wait: 1, ..Reply::default() };
        assert!(session.execute(&mut reply, true).await.unwrap());
        assert_eq!(sent(&session, ZConnectState::Eot(EndTransmission::Prot5)), 3);
        assert_eq!(session.io.read_u8().await.unwrap(), b'*');

        let mut bytes = wire(ZConnectState::Ack(BlockCode::Block3), &[]);
        bytes.extend(wire(ZConnectState::Block(BlockCode::Block4), &["Execute:J"]));
        bytes.extend(wire(ZConnectState::Eot(EndTransmission::End4), &[]).repeat(3));
        bytes.extend(wire(ZConnectState::Begin(ProtocolTransition::Prot6), &[]));
        let mut session = scripted(bytes);
        session.phase = Phase::Execute;
        assert!(session.execute(&mut Reply { wait: 1, ..Reply::default() }, true).await.is_err());
    }

    #[tokio::test]
    async fn outgoing_optional_modes_are_rejected_before_sending() {
        let mut session = scripted(Vec::new());
        session.phase = Phase::Ready;
        assert!(session.request(&ZConnectCommandBlock::default().get(mails::ALL)).await.is_err());
        assert!(session.request(&ZConnectCommandBlock::default().filereq("/INFO/LOGIN")).await.is_err());
        assert!(
            session
                .request(&ZConnectCommandBlock::default().get(mails::NEWS).put(mails::NEWS))
                .await
                .is_err()
        );
        assert!(session.io.inner.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn block_and_noise_limits_apply_before_parsing() {
        let mut session = scripted(vec![b'A'; 65537]);
        assert!(matches!(
            session.read_frame().await.unwrap_err().downcast_ref::<SessionError>(),
            Some(SessionError::Limit)
        ));
        let mut session = scripted(vec![0; 131073]);
        assert!(matches!(
            session.read_frame().await.unwrap_err().downcast_ref::<SessionError>(),
            Some(SessionError::Limit)
        ));
    }

    #[test]
    fn mandatory_crc_status_and_nonpanicking_parsers() {
        for bytes in ["\r\r", "Status:ACK1\r\r", "CRC:FFFF\r\r"] {
            assert!(Frame::default().parse_block(bytes).is_err());
        }
        let duplicate = wire(ZConnectState::Ack(BlockCode::Block1), &["Status:ACK1"]);
        assert!(Frame::default().parse_block(std::str::from_utf8(&duplicate).unwrap()).is_err());
        for line in ["Port:0", "Proto:1", "Arc:not-a-port ZIP", "Tel:"] {
            let bytes = wire(ZConnectState::Block(BlockCode::Block2), &[line]);
            assert!(ZConnectHeaderBlock::parse(std::str::from_utf8(&bytes).unwrap()).is_err());
        }
        let bytes = wire(ZConnectState::Block(BlockCode::Block2), &["Execute:INVALID"]);
        assert!(ZConnectCommandBlock::parse(std::str::from_utf8(&bytes).unwrap()).is_err());
    }

    #[test]
    fn global_ports_and_normative_arcer_names() {
        let bytes = wire(
            ZConnectState::Block(BlockCode::Block2),
            &["Port:2", "Arc:0 ZIP", "Proto:0 ZMODEM", "ArcerOut:ZIP"],
        );
        let header = ZConnectHeaderBlock::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(header.protocols(1), Some(&vec![TransferProtocol::ZModem]));
        assert_eq!(header.acer(1), Some(&vec![Acer::ZIP]));
        assert_eq!(header.acer_out(), Some(Acer::ZIP));
        assert!(header.display().contains("Proto:0 ZMODEM"));
        let bytes = wire(ZConnectState::Block(BlockCode::Block1), &["Delete:B", "Format:ZCONNECT"]);
        let block = ZConnectCommandBlock::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(block.commands(), &[ZConnectCmd::Delete(mails::NEWS), ZConnectCmd::Format("ZCONNECT".into())]);
    }

    #[tokio::test]
    async fn header_negotiation_is_not_parsed_as_commands() {
        let mut bytes = b"BEGIN\rBEGIN\r".to_vec();
        bytes.extend(wire(ZConnectState::Ack(BlockCode::Block1), &[]));
        bytes.extend(wire(
            ZConnectState::Block(BlockCode::Block2),
            &[
                "Sys:peer.example",
                "Sysop:sysop",
                "Port:1",
                "Tel:1 TCP",
                "Proto:0 ZMODEM",
                "Arc:0 ZIP",
                "Passwd:secret",
            ],
        ));
        bytes.extend(wire(ZConnectState::Tme(BlockCode::Block2), &[]));
        bytes.extend(wire(ZConnectState::Ack(BlockCode::Block3), &[]));
        bytes.extend(wire(ZConnectState::Block(BlockCode::Block4), &[]));
        bytes.extend(wire(ZConnectState::Tme(BlockCode::Block4), &[]));
        let mut session = scripted(bytes);
        session.login(Login::Direct, &identity()).await.unwrap();
        session.negotiate(&identity()).await.unwrap();
        assert!(session.phase == Phase::Ready);
        let output = session.io.inner.sent.lock().unwrap();
        assert!(output.iter().any(|bytes| String::from_utf8_lossy(bytes).contains("ArcerIn:ZIP\r")));
    }

    #[tokio::test]
    async fn optional_expected_sys_accepts_display_names_but_peer_sys_is_mandatory() {
        for (expected, peer_sys, succeeds) in [
            ("", Some("The Remote BBS (Public)"), true),
            ("the remote bbs (public)", Some("The Remote BBS (Public)"), true),
            ("Different BBS", Some("The Remote BBS (Public)"), false),
            ("", Some(""), false),
            ("", None, false),
            ("peer.example", Some(""), false),
            ("peer.example", None, false),
        ] {
            let identity = Identity {
                remote_system: expected,
                ..identity()
            };
            let mut bytes = b"BEGIN\r".to_vec();
            bytes.extend(wire(ZConnectState::Ack(BlockCode::Block1), &[]));
            let mut lines = vec!["Sysop:sysop", "Port:1", "Tel:1 TCP", "Proto:0 ZMODEM", "Arc:0 ZIP", "Passwd:secret"];
            let sys_line = peer_sys.map(|name| format!("Sys:{name}"));
            if let Some(line) = &sys_line {
                lines.push(line);
            }
            bytes.extend(wire(ZConnectState::Block(BlockCode::Block2), &lines));
            bytes.extend(wire(ZConnectState::Tme(BlockCode::Block2), &[]));
            bytes.extend(wire(ZConnectState::Ack(BlockCode::Block3), &[]));
            bytes.extend(wire(ZConnectState::Block(BlockCode::Block4), &[]));
            bytes.extend(wire(ZConnectState::Tme(BlockCode::Block4), &[]));
            let mut session = scripted(bytes);
            session.login(Login::Direct, &identity).await.unwrap();
            assert_eq!(session.negotiate(&identity).await.is_ok(), succeeds, "expected={expected:?}, peer={peer_sys:?}");
        }
    }

    #[tokio::test]
    async fn invalid_expected_sys_is_rejected_at_both_public_entry_points() {
        for name in [
            "bad\rname".into(),
            "bad\nname".into(),
            "bad\tname".into(),
            "bad\x7fname".into(),
            "Büro".into(),
            "X".repeat(256),
        ] {
            let identity = Identity {
                remote_system: &name,
                ..identity()
            };
            let mut session = scripted(b"BEGIN\r".to_vec());
            assert!(session.login(Login::Direct, &identity).await.is_err());
            assert!(session.negotiate(&identity).await.is_err());
            assert!(session.io.inner.sent.lock().unwrap().is_empty());
        }
        assert!(validate_expected_system(&"X".repeat(255)).is_ok());
    }

    #[tokio::test]
    async fn janus_login_keeps_coalesced_begin_and_header_tail() {
        let mut bytes = b"Username:Systemname:Password:BEGIN\r".to_vec();
        bytes.extend(wire(ZConnectState::Ack(BlockCode::Block1), &[]));
        let mut session = scripted(bytes);
        session.login(Login::Janus, &identity()).await.unwrap();
        assert_eq!(
            *session.io.inner.sent.lock().unwrap(),
            [b"JANUS\r".to_vec(), b"point\r".to_vec(), b"secret\r".to_vec()]
        );
        assert_eq!(session.read_frame().await.unwrap().state, ZConnectState::Ack(BlockCode::Block1));
    }

    #[test]
    fn unsupported_capabilities_and_response_flags_are_rejected() {
        let mut header = ZConnectHeaderBlock::default();
        header.set_system("peer.example");
        header.add_acer(0, Acer::ZIP);
        assert!(validate_header(&header, &identity()).is_err());
        header.add_protocol(0, TransferProtocol::ZModem);
        assert!(validate_header(&header, &identity()).is_ok());
        header.add_crypt(0, super::super::header::Crypt::PGP);
        assert!(validate_header(&header, &identity()).is_err());
        for line in [
            "Put:PB",
            "Put:X",
            "Execute:L",
            "Format:RFC1036",
            "Filesend:secret",
            "Wait:999999",
            "Bytes:999999999999",
        ] {
            let frame = Frame {
                state: ZConnectState::Block(BlockCode::Block2),
                lines: vec![line.to_uppercase()],
            };
            assert!(Reply::default().merge(&frame, &limits()).is_err(), "{line}");
        }
    }

    #[tokio::test]
    async fn retransmit_overrides_the_post_transfer_ack_boundary() {
        let mut bytes = wire(ZConnectState::Ack(BlockCode::Block1), &[]);
        bytes.extend(wire(ZConnectState::Block(BlockCode::Block2), &["Retransmit:CRC failed"]));
        bytes.extend(wire(ZConnectState::Tme(BlockCode::Block2), &[]));
        let mut session = scripted(bytes);
        session.phase = Phase::Ready;
        let error = session.request(&ZConnectCommandBlock::default().get(mails::NEWS)).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<SessionError>(), Some(SessionError::Retransmit)));
    }

    #[test]
    fn safe_names_size_crc_and_durable_collision_free_storage() {
        for name in ["../mail.zip", "/mail.zip", "C:\\mail.zip", "a/b.zip", "a\r.zip", "mail.exe", ".hidden.zip"] {
            assert!(safe_zip_name(name).is_err());
        }
        assert!(safe_zip_name("MAIL.ZI2").is_ok());
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("packet.zip");
        let bytes = b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        fs::write(&source, bytes).unwrap();
        validate_zip(&source, Some(bytes.len() as u64), Some(crate::crc::get_crc32(bytes)), 100).unwrap();
        assert!(validate_zip(&source, Some(2), None, 100).is_err());
        assert!(validate_zip(&source, None, Some(0), 100).is_err());
        assert!(validate_zip(&source, None, None, 2).is_err());
        let a = durable_zip(&source, directory.path()).unwrap();
        let b = durable_zip(&source, directory.path()).unwrap();
        assert_ne!(a, b);
        assert_eq!(fs::read(a).unwrap(), bytes);
        assert!(source.exists());
    }

    #[tokio::test]
    async fn real_zmodem_roundtrip_keeps_archive_and_post_transfer_control() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("mail.zip");
        let contents = b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        fs::write(&source, contents).unwrap();
        let inbound = directory.path().join("inbound");
        let (local, remote) = crate::connection::channel::ChannelConnection::create_pair();
        let mut sender = Session::new(remote, limits()).unwrap();
        let mut receiver = Session::new(local, limits()).unwrap();
        sender.phase = Phase::Transfer;
        receiver.phase = Phase::Transfer;
        let reply = Reply {
            bytes: Some(contents.len() as u64),
            crc: Some(crate::crc::get_crc32(contents)),
            ..Reply::default()
        };
        let sending = async {
            sender.send_zip(&source).await.unwrap();
            sender.control(ZConnectState::Nak0).await.unwrap();
            sender.shutdown().await.unwrap();
        };
        let receiving = async {
            let path = receiver.receive_zip(&inbound, &reply).await.unwrap();
            assert_eq!(fs::read(&path).unwrap(), contents);
            assert_eq!(receiver.read_frame().await.unwrap().state, ZConnectState::Nak0);
            assert!(receiver.request(&ZConnectCommandBlock::default().get(mails::NEWS)).await.is_err());
            receiver.shutdown().await.unwrap();
            assert!(path.exists());
        };
        timeout(Duration::from_secs(15), async {
            tokio::join!(sending, receiving);
        })
        .await
        .unwrap();
        assert!(source.exists(), "transport success must not retire outbound mail");
    }
}
