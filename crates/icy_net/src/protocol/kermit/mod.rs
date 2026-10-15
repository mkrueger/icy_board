//! Classic Kermit: short packets, stop-and-wait, and type-1 block checks.
//! Wire format: <https://www.kermitproject.org/kproto.pdf>.
mod packet;

use std::{
    collections::VecDeque,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    time::Duration,
};

use async_trait::async_trait;
use packet::{PARAMETERS, Packet, Parameters, Reader};
use tempfile::NamedTempFile;
use tokio::time::{Instant, timeout};

use super::{Protocol, TransferInformation, TransferState};
use crate::{Connection, NetError};

const RETRIES: usize = 10;
const LINGER: Duration = Duration::from_secs(11);

#[derive(Debug, thiserror::Error)]
pub enum KermitError {
    #[error("Invalid Kermit packet")]
    InvalidPacket,
    #[error("Invalid Kermit initialization parameters")]
    InvalidParameters,
    #[error("Unexpected Kermit packet type {0:#x}")]
    UnexpectedPacket(u8),
    #[error("Kermit retry limit exceeded")]
    RetryLimit,
    #[error("Remote Kermit error: {0}")]
    Remote(String),
    #[error("Kermit transfer cancelled")]
    Cancelled,
    #[error("Invalid or overlong Kermit filename")]
    FileName,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    SendInit,
    SendFile,
    SendData,
    SendEof,
    SendBreak,
    ReceiveInit,
    ReceiveFile,
    ReceiveData,
    Linger,
    Done,
}

/// Binary batch transfers; received files are kept in temporary paths on completion.
/// Long packets, sliding windows, attributes, and server commands are not advertised.
pub struct Kermit {
    phase: Phase,
    sending: bool,
    peer: Parameters,
    reader: Reader,
    sequence: u8,
    retries: usize,
    deadline: Instant,
    pending: Vec<u8>,
    last_received: Option<Packet>,
    files: VecDeque<PathBuf>,
    source: Option<BufReader<File>>,
    source_path: PathBuf,
    destination: Option<NamedTempFile>,
    data_count: usize,
    discard: bool,
    cancel_batch: bool,
    receive_limit: usize,
    received_files: usize,
}

impl Default for Kermit {
    fn default() -> Self {
        Self::new()
    }
}

impl Kermit {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            sending: false,
            peer: Parameters::default(),
            reader: Reader::default(),
            sequence: 0,
            retries: 0,
            deadline: Instant::now(),
            pending: Vec::new(),
            last_received: None,
            files: VecDeque::new(),
            source: None,
            source_path: PathBuf::new(),
            destination: None,
            data_count: 0,
            discard: false,
            cancel_batch: false,
            receive_limit: usize::MAX,
            received_files: 0,
        }
    }

    fn reset(&mut self, sending: bool) {
        let limit = self.receive_limit;
        *self = Self::new();
        self.receive_limit = limit;
        self.sending = sending;
        self.deadline = Instant::now() + self.peer.timeout;
    }

    fn info<'a>(&self, state: &'a mut TransferState) -> &'a mut TransferInformation {
        if self.sending { &mut state.send_state } else { &mut state.recieve_state }
    }

    fn stop(&mut self) {
        self.phase = Phase::Done;
        self.source = None;
        self.destination = None;
        self.pending.clear();
    }

    async fn send(&mut self, com: &mut dyn Connection, kind: u8, data: Vec<u8>) -> crate::Result<()> {
        self.pending = self.peer.frame(&Packet {
            sequence: self.sequence,
            kind,
            data,
        })?;
        com.send(&self.pending).await?;
        self.deadline = Instant::now() + self.peer.timeout;
        Ok(())
    }

    async fn retry(&mut self, com: &mut dyn Connection, state: &mut TransferState, reason: &str, damaged: bool) -> crate::Result<()> {
        self.retries += 1;
        self.info(state).log_warning(reason);
        if self.retries > RETRIES {
            return Err(KermitError::RetryLimit.into());
        }
        self.reader.clear();
        if !self.sending && (damaged || self.pending.is_empty()) {
            let nak = self.peer.frame(&Packet {
                sequence: self.sequence,
                kind: b'N',
                data: Vec::new(),
            })?;
            com.send(&nak).await?;
        } else {
            com.send(&self.pending).await?;
        }
        self.deadline = Instant::now() + self.peer.timeout;
        Ok(())
    }

    async fn next_file(&mut self, com: &mut dyn Connection, state: &mut TransferState) -> crate::Result<()> {
        if self.cancel_batch {
            self.files.clear();
        }
        let Some(path) = self.files.pop_front() else {
            self.phase = Phase::SendBreak;
            return self.send(com, b'B', Vec::new()).await;
        };
        let name = path.file_name().and_then(|n| n.to_str()).ok_or(KermitError::FileName)?;
        let (encoded, used) = self.peer.outgoing().encode(name.as_bytes(), self.peer.max_len - 3);
        if used != name.len() || name.is_empty() {
            return Err(KermitError::FileName.into());
        }
        let file = File::open(&path)?;
        state.send_state.file_name = name.to_string();
        state.send_state.file_size = file.metadata()?.len();
        state.send_state.reset_cur_transfer();
        self.source = Some(BufReader::new(file));
        self.source_path = path;
        self.discard = false;
        self.phase = Phase::SendFile;
        self.send(com, b'F', encoded).await
    }

    async fn next_data(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        if !self.discard {
            let source = self.source.as_mut().ok_or(KermitError::UnexpectedPacket(b'D'))?;
            let (encoded, count) = self.peer.outgoing().encode(source.fill_buf()?, self.peer.max_len - 3);
            source.consume(count);
            self.data_count = count;
            if count > 0 {
                self.phase = Phase::SendData;
                return self.send(com, b'D', encoded).await;
            }
        }
        self.phase = Phase::SendEof;
        self.send(com, b'Z', if self.discard { vec![b'D'] } else { Vec::new() }).await
    }

    async fn sender_packet(&mut self, com: &mut dyn Connection, state: &mut TransferState, packet: Packet) -> crate::Result<()> {
        let acknowledged = packet.kind == b'Y' && packet.sequence == self.sequence;
        // Send-Init must have a real ACK carrying the peer's parameters.
        let next_nak = packet.kind == b'N' && packet.sequence == (self.sequence + 1) % 64 && self.phase != Phase::SendInit;
        if !acknowledged && !next_nak {
            if packet.kind == b'N' && packet.sequence == self.sequence {
                return self.retry(com, state, "Kermit packet rejected; retransmitting", false).await;
            }
            if !matches!(packet.kind, b'Y' | b'N') {
                return Err(KermitError::UnexpectedPacket(packet.kind).into());
            }
            return Ok(());
        }
        if self.phase == Phase::SendInit {
            self.peer = Parameters::parse(&packet.data, false)?;
            state.send_state.check_size = format!("Checksum/{}", self.peer.max_len);
        } else if acknowledged && matches!(self.phase, Phase::SendFile | Phase::SendData) {
            let response = self.peer.incoming.decode(&packet.data)?;
            if matches!(response.first(), Some(b'X' | b'Z')) {
                self.discard = true;
                self.cancel_batch = response[0] == b'Z';
                state.send_state.log_warning("Remote Kermit cancelled the file");
            }
        }
        self.retries = 0;
        self.sequence = (self.sequence + 1) % 64;
        match self.phase {
            Phase::SendInit => self.next_file(com, state).await?,
            Phase::SendFile => self.next_data(com).await?,
            Phase::SendData => {
                state.send_state.cur_bytes_transfered += self.data_count as u64;
                state.send_state.total_bytes_transfered += self.data_count as u64;
                self.next_data(com).await?;
            }
            Phase::SendEof => {
                self.source = None;
                if !self.discard {
                    state.send_state.finish_file(self.source_path.clone());
                }
                self.next_file(com, state).await?;
            }
            Phase::SendBreak => self.stop(),
            _ => return Err(KermitError::UnexpectedPacket(packet.kind).into()),
        }
        Ok(())
    }

    async fn receiver_packet(&mut self, com: &mut dyn Connection, state: &mut TransferState, packet: Packet) -> crate::Result<()> {
        if self.last_received.as_ref() == Some(&packet) {
            if self.phase == Phase::Linger {
                com.send(&self.pending).await?;
                return Ok(());
            }
            return self.retry(com, state, "Duplicate Kermit packet; repeating ACK", false).await;
        }
        if packet.sequence != self.sequence {
            return self.retry(com, state, "Unexpected Kermit sequence; requesting retransmission", true).await;
        }
        let mut response = Vec::new();
        match (self.phase, packet.kind) {
            (Phase::ReceiveInit, b'S') => {
                self.peer = Parameters::parse(&packet.data, true)?;
                response = self.peer.response();
                self.phase = Phase::ReceiveFile;
            }
            (Phase::ReceiveFile, b'F') => {
                let decoded = self.peer.incoming.decode(&packet.data)?;
                let name = String::from_utf8_lossy(&decoded);
                let name = name.rsplit(['/', '\\']).next().ok_or(KermitError::FileName)?;
                if name.is_empty() || matches!(name, "." | "..") || name.chars().any(char::is_control) {
                    return Err(KermitError::FileName.into());
                }
                state.recieve_state.file_name = name.to_string();
                state.recieve_state.file_size = 0;
                state.recieve_state.reset_cur_transfer();
                self.discard = self.received_files >= self.receive_limit;
                if self.discard {
                    response.push(b'X');
                    state.recieve_state.log_warning("Kermit receive file limit reached; skipping file");
                } else {
                    self.destination = Some(NamedTempFile::new()?);
                }
                self.phase = Phase::ReceiveData;
            }
            (Phase::ReceiveData, b'D') => {
                let data = self.peer.incoming.decode(&packet.data)?;
                if self.discard {
                    response.push(b'X');
                } else {
                    self.destination.as_mut().ok_or(KermitError::UnexpectedPacket(b'D'))?.write_all(&data)?;
                    state.recieve_state.cur_bytes_transfered += data.len() as u64;
                    state.recieve_state.total_bytes_transfered += data.len() as u64;
                    state.recieve_state.file_size = state.recieve_state.cur_bytes_transfered;
                }
            }
            (Phase::ReceiveData, b'Z') => {
                let data = self.peer.incoming.decode(&packet.data)?;
                if data.first() == Some(&b'D') {
                    self.destination = None;
                    state.recieve_state.log_warning("Remote Kermit discarded the file");
                } else if let Some(mut file) = self.destination.take() {
                    file.flush()?;
                    let (_, path) = file.keep()?;
                    state.recieve_state.finish_file(path);
                    self.received_files += 1;
                }
                self.phase = Phase::ReceiveFile;
            }
            (Phase::ReceiveFile, b'B') => self.phase = Phase::Linger,
            _ => return Err(KermitError::UnexpectedPacket(packet.kind).into()),
        }
        self.send(com, b'Y', response).await?;
        self.last_received = Some(packet);
        self.sequence = (self.sequence + 1) % 64;
        self.retries = 0;
        if self.phase == Phase::Linger {
            self.deadline = Instant::now() + LINGER;
        }
        Ok(())
    }

    async fn step(&mut self, com: &mut dyn Connection, state: &mut TransferState) -> crate::Result<()> {
        if state.request_cancel {
            return Err(KermitError::Cancelled.into());
        }
        let finishing = self.phase == Phase::Linger;
        if Instant::now() >= self.deadline {
            if finishing {
                self.stop();
                return Ok(());
            }
            return self.retry(com, state, "Kermit response timed out; retransmitting", false).await;
        }
        let wait = self.deadline.saturating_duration_since(Instant::now()).min(Duration::from_secs(1));
        let packet = match timeout(wait, self.reader.read(com, finishing)).await {
            Err(_) => return Ok(()),
            Ok(Ok(None)) => {
                self.stop();
                return Ok(());
            }
            Ok(Ok(Some(packet))) => packet,
            Ok(Err(error)) => {
                if finishing && matches!(error.downcast_ref::<NetError>(), Some(NetError::ConnectionClosed)) {
                    self.stop();
                    return Ok(());
                }
                if matches!(error.downcast_ref::<KermitError>(), Some(KermitError::InvalidPacket)) {
                    if finishing {
                        state.recieve_state.log_warning("Damaged Kermit packet during completion; repeating ACK");
                        return com.send(&self.pending).await;
                    }
                    return self.retry(com, state, "Damaged Kermit packet; requesting retransmission", true).await;
                }
                return Err(error);
            }
        };
        if packet.kind == b'E' {
            let message = self.peer.incoming.decode(&packet.data)?;
            return Err(KermitError::Remote(String::from_utf8_lossy(&message).into_owned()).into());
        }
        if self.sending {
            self.sender_packet(com, state, packet).await
        } else {
            self.receiver_packet(com, state, packet).await
        }
    }
}

#[async_trait]
impl Protocol for Kermit {
    fn set_receive_limit(&mut self, limit: usize) -> bool {
        self.receive_limit = limit;
        true
    }

    async fn initiate_send(&mut self, com: &mut dyn Connection, files: &[PathBuf]) -> crate::Result<TransferState> {
        self.reset(true);
        self.files = files.iter().cloned().collect();
        self.phase = Phase::SendInit;
        if let Err(error) = self.send(com, b'S', PARAMETERS.to_vec()).await {
            self.stop();
            return Err(error);
        }
        let mut state = TransferState::new("Kermit".into());
        state.send_state.check_size = "Checksum/94".into();
        Ok(state)
    }

    async fn initiate_recv(&mut self, _com: &mut dyn Connection) -> crate::Result<TransferState> {
        self.reset(false);
        self.phase = Phase::ReceiveInit;
        let mut state = TransferState::new("Kermit".into());
        state.recieve_state.check_size = "Checksum/94".into();
        Ok(state)
    }

    async fn update_transfer(&mut self, com: &mut dyn Connection, state: &mut TransferState) -> crate::Result<()> {
        if matches!(self.phase, Phase::Done | Phase::Idle) {
            state.is_finished = true;
            return Ok(());
        }
        let result = self.step(com, state).await;
        if let Err(error) = &result {
            self.info(state).log_error(error.to_string());
            if !matches!(error.downcast_ref::<KermitError>(), Some(KermitError::Remote(_))) {
                let (message, _) = self.peer.outgoing().encode(b"Transfer failed", self.peer.max_len - 3);
                if let Err(send_error) = self.send(com, b'E', message).await {
                    self.info(state).log_warning(format!("Could not send Kermit error: {send_error}"));
                }
            }
            self.stop();
        }
        state.is_finished = self.phase == Phase::Done;
        state.current_state = match self.phase {
            Phase::SendInit | Phase::ReceiveInit => "Negotiating",
            Phase::SendFile | Phase::ReceiveFile => "File header",
            Phase::SendData | Phase::ReceiveData => "Transferring",
            Phase::SendEof => "End of file",
            Phase::SendBreak | Phase::Linger => "Finishing",
            _ => "Finished",
        };
        result
    }

    async fn cancel_transfer(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        if self.phase == Phase::Done {
            return Ok(());
        }
        let (message, _) = self.peer.outgoing().encode(b"Cancelled", self.peer.max_len - 3);
        let result = self.send(com, b'E', message).await;
        self.stop();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::channel::ChannelConnection;

    #[tokio::test(start_paused = true)]
    async fn partial_receive_files_are_deleted_on_cancel_error_and_reinitialization() {
        for failure in 0..3 {
            let (mut local, mut remote) = ChannelConnection::create_pair();
            let mut protocol = Kermit::new();
            let mut state = protocol.initiate_recv(&mut local).await.unwrap();
            for (sequence, kind, data) in [(0, b'S', PARAMETERS), (1, b'F', b"partial.bin".as_slice()), (2, b'D', b"partial".as_slice())] {
                remote
                    .send(
                        &Parameters::default()
                            .frame(&Packet {
                                sequence,
                                kind,
                                data: data.to_vec(),
                            })
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                protocol.update_transfer(&mut local, &mut state).await.unwrap();
            }
            let path = protocol.destination.as_ref().unwrap().path().to_path_buf();
            assert!(path.exists());
            match failure {
                0 => protocol.cancel_transfer(&mut local).await.unwrap(),
                1 => {
                    remote
                        .send(
                            &Parameters::default()
                                .frame(&Packet {
                                    sequence: 3,
                                    kind: b'E',
                                    data: b"Aborted".to_vec(),
                                })
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                    assert!(protocol.update_transfer(&mut local, &mut state).await.is_err());
                }
                _ => {
                    protocol.initiate_recv(&mut local).await.unwrap();
                }
            }
            assert!(!path.exists());
            assert!(state.recieve_state.finished_files.is_empty());
        }
    }
}
