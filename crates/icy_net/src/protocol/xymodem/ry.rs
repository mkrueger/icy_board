use std::{io::Write, time::Duration};

use tempfile::NamedTempFile;
use tokio::time::timeout;

use super::{Checksum, XYModemConfiguration, constants::DEFAULT_BLOCK_LENGTH, err::XYModemError, get_checksum, remove_cpm_eof, truncate_to_file_size};
use crate::{
    Connection,
    crc::get_crc16,
    protocol::{
        TransferState, str_from_null_terminated_utf8_unchecked,
        xymodem::constants::{ACK, CAN, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
    },
};

/// Timeout for waiting for a byte from the sender (3 seconds)
const READ_TIMEOUT: Duration = Duration::from_secs(3);

/// Maximum number of retries before giving up
const MAX_RETRIES: usize = 5;

/// Section 7.2 gives up on a block after ten unsuccessful attempts.
const MAX_ERRORS: usize = 10;

#[derive(Debug)]
pub enum RecvState {
    None,

    StartReceive(usize),
    ReadYModemHeader(usize, usize),
    ReadBlock(usize, usize),
    ReadBlockStart(u8, usize),
}

/// specification: <http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt>
pub struct Ry {
    configuration: XYModemConfiguration,
    errors: usize,
    recv_state: RecvState,
    cur_out_file: Option<NamedTempFile>,
    pending_finished_file: Option<NamedTempFile>,
    last_block_len: usize,
    expected_block_num: u8,
    declared_size: Option<u64>,
    file_header: Option<Vec<u8>>,
    received_data: bool,
    last_file_finished: bool,
    previous_can: bool,
}

impl Ry {
    pub fn new(configuration: XYModemConfiguration) -> Self {
        Ry {
            configuration,
            recv_state: RecvState::None,
            errors: 0,
            last_block_len: 0,
            expected_block_num: 1,
            cur_out_file: None,
            pending_finished_file: None,
            declared_size: None,
            file_header: None,
            received_data: false,
            last_file_finished: false,
            previous_can: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.recv_state, RecvState::None)
    }

    pub async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        let result = self.update_transfer_inner(com, transfer_state).await;
        if result.is_err() {
            self.recv_state = RecvState::None;
            self.cur_out_file = None;
            self.pending_finished_file = None;
            transfer_state.is_finished = true;
        }
        result
    }

    async fn update_transfer_inner(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        {
            let transfer_info = &mut transfer_state.recieve_state;
            transfer_info.errors = self.errors;
            transfer_info.check_size = self.configuration.get_check_and_size();
        }

        match self.recv_state {
            RecvState::None => {}

            RecvState::StartReceive(retries) => {
                transfer_state.current_state = "Start receiving...";
                if retries == 0 {
                    let mode_str = match self.configuration.checksum_mode {
                        Checksum::Default => "checksum",
                        Checksum::CRC16 => "CRC16",
                    };
                    let variant_str = format!("{:?}", self.configuration.variant);
                    transfer_state
                        .recieve_state
                        .log_info(format!("Starting {} receive with {} verification", variant_str, mode_str));
                }

                let start = match timeout(READ_TIMEOUT, self.read_control(com)).await {
                    Ok(Ok(byte)) => byte,
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(_) => {
                        transfer_state
                            .recieve_state
                            .log_warning(format!("Timeout waiting for start byte (retry {})", retries));
                        if retries >= MAX_RETRIES {
                            transfer_state.recieve_state.log_error("Too many timeouts waiting for start");
                            self.cancel(com).await?;
                            return Err(XYModemError::Timeout.into());
                        }
                        // Fallback chain: Streaming (G) -> CRC (C) -> Checksum (NAK)
                        if retries == 1 && self.configuration.streaming_enabled {
                            transfer_state.recieve_state.log_info("No streaming response, falling back to CRC mode");
                            self.configuration.streaming_enabled = false;
                        } else if retries == 2 && self.configuration.checksum_mode == Checksum::CRC16 {
                            transfer_state.recieve_state.log_info("No CRC response, falling back to checksum mode");
                            self.configuration.checksum_mode = Checksum::Default;
                        }
                        self.await_data(com).await?;
                        self.recv_state = RecvState::StartReceive(retries + 1);
                        return Ok(());
                    }
                };

                if start == SOH || start == STX {
                    let len = if start == SOH { DEFAULT_BLOCK_LENGTH } else { EXT_BLOCK_LENGTH };
                    if self.configuration.is_ymodem() {
                        self.recv_state = RecvState::ReadYModemHeader(len, retries);
                    } else {
                        self.cur_out_file = Some(NamedTempFile::new()?);
                        transfer_state.recieve_state.file_name = String::new();
                        self.expected_block_num = 1;
                        self.recv_state = RecvState::ReadBlock(len, 0);
                    }
                } else if start == EOT && !self.configuration.is_ymodem() {
                    // Empty XMODEM files have no SOH/STX block at all.
                    self.cur_out_file = Some(NamedTempFile::new()?);
                    self.receive_eot(com, transfer_state).await?;
                } else if start == EOT && self.last_file_finished {
                    // The sender lost the final EOT ACK. Do not count the file twice.
                    com.send(&[ACK, self.start_byte()]).await?;
                } else {
                    transfer_state
                        .recieve_state
                        .log_warning(format!("Invalid start byte: 0x{:02X} (retry {})", start, retries));
                    if retries < 3 {
                        self.await_data(com).await?;
                    } else if retries == 4 {
                        transfer_state.recieve_state.log_info("Sending NAK to request retransmission");
                        com.send(&[NAK]).await?;
                    } else {
                        transfer_state.recieve_state.log_error("Too many retries waiting for start");
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesStarting.into());
                    }
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(retries + 1);
                }
            }

            RecvState::ReadYModemHeader(len, retries) => {
                transfer_state.current_state = "Get header...";
                if retries > 0 {
                    transfer_state
                        .recieve_state
                        .log_warning(format!("Retrying YModem header read (attempt {})", retries + 1));
                }

                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                let mut block = vec![0; 2 + len + chksum_size];
                match timeout(READ_TIMEOUT, com.read_exact(&mut block)).await {
                    Ok(result) => result?,
                    Err(_) => {
                        self.cancel(com).await?;
                        return Err(XYModemError::Timeout.into());
                    }
                }

                if block[0] != block[1] ^ 0xFF {
                    transfer_state
                        .recieve_state
                        .log_error(format!("Block number check failed: {:02X} != {:02X}^FF", block[0], block[1]));
                    self.errors += 1;
                    if self.configuration.is_streaming() || retries >= MAX_ERRORS {
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::StartReceive(retries + 1);
                    return Ok(());
                }

                if block[0] != 0 {
                    transfer_state
                        .recieve_state
                        .log_error(format!("YModem file header used block number {} instead of 0", block[0]));
                    self.cancel(com).await?;
                    return Err(XYModemError::OutOfSyncBlock(0, block[0]).into());
                }

                let body = &block[2..];
                if !self.check_crc(body) {
                    transfer_state.recieve_state.log_error("YModem header CRC/checksum verification failed");
                    self.errors += 1;
                    if self.configuration.is_streaming() {
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    if retries >= MAX_ERRORS {
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    com.send(&[NAK]).await?;
                    // The retransmission starts with a fresh SOH byte.
                    self.recv_state = RecvState::StartReceive(retries + 1);
                    return Ok(());
                }

                let block = &body[..len]; // Exclude CRC bytes from metadata parsing.
                if block[0] == 0 {
                    transfer_state.recieve_state.log_info("End of batch transfer detected");
                    com.send(&[ACK]).await?;
                    self.recv_state = RecvState::None;
                    return Ok(());
                }

                let (file_name, declared_size) = match parse_file_info(block) {
                    Ok(info) => info,
                    Err(error) => {
                        self.cancel(com).await?;
                        return Err(error.into());
                    }
                };
                let file_size = declared_size.unwrap_or(0);

                transfer_state
                    .recieve_state
                    .log_info(format!("Receiving file '{}' ({} bytes)", file_name, file_size));
                transfer_state.recieve_state.file_name = file_name;
                transfer_state.recieve_state.file_size = file_size;
                self.cur_out_file = Some(NamedTempFile::new()?);
                self.expected_block_num = 1;
                self.declared_size = declared_size;
                self.file_header = Some(block.to_vec());
                self.received_data = false;
                self.last_file_finished = false;
                self.last_block_len = 0;
                self.errors = 0;
                transfer_state.recieve_state.reset_cur_transfer();

                if self.configuration.is_ymodem() {
                    let start_byte = self.start_byte();
                    if self.configuration.is_streaming() {
                        transfer_state.recieve_state.log_info("Sending G for YModem-G data blocks");
                        com.send(&[start_byte]).await?;
                    } else {
                        transfer_state.recieve_state.log_info("Sending ACK+C for YModem data blocks");
                        com.send(&[ACK, start_byte]).await?;
                    }
                } else {
                    com.send(&[ACK]).await?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }

            RecvState::ReadBlockStart(step, retries) => {
                if step == 0 {
                    let start = match timeout(READ_TIMEOUT, self.read_control(com)).await {
                        Ok(Ok(byte)) => byte,
                        Ok(Err(e)) => {
                            return Err(e);
                        }
                        Err(_) => {
                            transfer_state
                                .recieve_state
                                .log_warning(format!("Timeout waiting for block start (retry {})", retries));
                            if retries >= MAX_RETRIES || self.configuration.is_streaming() {
                                transfer_state.recieve_state.log_error("Too many timeouts waiting for block");
                                self.cancel(com).await?;
                                return Err(XYModemError::Timeout.into());
                            }
                            let request = if self.file_header.is_some() && !self.received_data {
                                self.start_byte()
                            } else {
                                NAK
                            };
                            com.send(&[request]).await?;
                            self.errors += 1;
                            self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                            return Ok(());
                        }
                    };

                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                    } else if start == EOT {
                        self.receive_eot(com, transfer_state).await?;
                    } else {
                        transfer_state
                            .recieve_state
                            .log_warning(format!("Invalid block start byte: 0x{:02X} (retry {})", start, retries));
                        if retries < 5 && !self.configuration.is_streaming() {
                            com.send(&[NAK]).await?;
                        } else {
                            transfer_state.recieve_state.log_error("Too many retries reading block start");
                            self.cancel(com).await?;
                            return Err(XYModemError::TooManyRetriesReadingBlock.into());
                        }
                        self.errors += 1;
                        self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    }
                } else if step == 1 {
                    let eot = match timeout(READ_TIMEOUT, self.read_control(com)).await {
                        Ok(Ok(byte)) => byte,
                        Ok(Err(e)) => {
                            return Err(e);
                        }
                        Err(_) => {
                            transfer_state
                                .recieve_state
                                .log_warning(format!("Timeout waiting for second EOT (retry {})", retries + 1));
                            if retries >= MAX_RETRIES {
                                self.cancel(com).await?;
                                return Err(XYModemError::Timeout.into());
                            }
                            com.send(&[NAK]).await?;
                            self.recv_state = RecvState::ReadBlockStart(1, retries + 1);
                            return Ok(());
                        }
                    };
                    if eot != EOT {
                        transfer_state
                            .recieve_state
                            .log_warning(format!("Expected second EOT but received: 0x{:02X}", eot));
                        self.cancel(com).await?;
                        return Err(XYModemError::InvalidResponse(eot).into());
                    }
                    transfer_state.recieve_state.log_info("Second EOT confirmed");

                    if self.configuration.is_ymodem() {
                        transfer_state.recieve_state.log_info("Ready for next file in batch");
                        com.send(&[ACK, self.start_byte()]).await?;
                        self.finish_received_file(transfer_state)?;
                    } else {
                        com.send(&[ACK]).await?;
                    }
                    self.recv_state = RecvState::StartReceive(0);
                }
            }

            RecvState::ReadBlock(len, retries) => {
                transfer_state.current_state = "Receiving data...";
                if retries > 0 {
                    transfer_state
                        .recieve_state
                        .log_warning(format!("Retrying block read (attempt {}, {} bytes)", retries + 1, len));
                }

                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                let mut block = vec![0; 2 + len + chksum_size];
                match timeout(READ_TIMEOUT, com.read_exact(&mut block)).await {
                    Ok(result) => result?,
                    Err(_) => {
                        self.cancel(com).await?;
                        return Err(XYModemError::Timeout.into());
                    }
                }

                let block_num = block[0];
                let block_num_inv = block[1];

                if block_num != block_num_inv ^ 0xFF {
                    transfer_state.recieve_state.log_error(format!(
                        "Block number verification failed: {:02X} != {:02X}^FF (block {})",
                        block_num, block_num_inv, block_num
                    ));
                    self.errors += 1;

                    if self.configuration.is_streaming() || self.errors >= MAX_ERRORS {
                        self.cancel(com).await?;
                        return Err(XYModemError::OutOfSyncBlock(block_num, block_num_inv ^ 0xFF).into());
                    }
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    return Ok(());
                }

                let block = &block[2..];
                if !self.check_crc(block) {
                    transfer_state.recieve_state.log_error(format!(
                        "CRC/checksum verification failed for block {} (error count: {})",
                        block_num,
                        self.errors + 1
                    ));
                    self.errors += 1;
                    if self.configuration.is_streaming() {
                        transfer_state.recieve_state.log_error("Streaming transfer cannot retransmit, aborting");
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesReadingBlock.into());
                    }
                    if self.errors >= MAX_ERRORS {
                        transfer_state.recieve_state.log_error("Too many block errors, aborting");
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesReadingBlock.into());
                    }
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    return Ok(());
                }

                // If the block-0 ACK or following C/G/NAK was lost, repeat both
                // responses. Do not reopen/reset the already accepted file.
                if !self.received_data && block_num == 0 && self.configuration.is_ymodem() {
                    if self.file_header.as_deref() != Some(&block[..len]) {
                        self.cancel(com).await?;
                        return Err(XYModemError::InvalidFileInfo("changed retransmitted header").into());
                    }
                    if self.configuration.is_streaming() {
                        com.send(&[self.start_byte()]).await?;
                    } else {
                        com.send(&[ACK, self.start_byte()]).await?;
                    }
                    self.recv_state = RecvState::ReadBlockStart(0, 0);
                    return Ok(());
                }

                // Section 7.3.2: the block is the expected one, a repeat of the one before it,
                // or the transfer has lost synchronisation for good.
                if self.received_data && block_num == self.expected_block_num.wrapping_sub(1) {
                    transfer_state
                        .recieve_state
                        .log_warning(format!("Block {} arrived twice, the acknowledgement for it was lost", block_num));
                    if !self.configuration.is_streaming() {
                        com.send(&[ACK]).await?;
                    }
                    self.recv_state = RecvState::ReadBlockStart(0, 0);
                    return Ok(());
                }
                if block_num != self.expected_block_num {
                    transfer_state
                        .recieve_state
                        .log_error(format!("Expected block {} but received {}, aborting", self.expected_block_num, block_num));
                    self.cancel(com).await?;
                    return Err(XYModemError::OutOfSyncBlock(self.expected_block_num, block_num).into());
                }

                self.last_block_len = len;
                let data_len = if let Some(size) = self.declared_size {
                    let remaining = size.saturating_sub(transfer_state.recieve_state.cur_bytes_transfered);
                    if remaining == 0 {
                        self.cancel(com).await?;
                        return Err(XYModemError::FileTooLong.into());
                    }
                    remaining.min(len as u64) as usize
                } else {
                    len
                };
                if let Some(named_file) = &mut self.cur_out_file {
                    named_file.as_file_mut().write_all(&block[..data_len])?;
                    transfer_state.recieve_state.total_bytes_transfered += data_len as u64;
                    transfer_state.recieve_state.cur_bytes_transfered += data_len as u64;
                } else {
                    transfer_state.recieve_state.log_error("No file open for writing block data");
                    return Err(XYModemError::NoFileOpen.into());
                }
                self.expected_block_num = self.expected_block_num.wrapping_add(1);
                self.received_data = true;
                self.errors = 0;

                if !self.configuration.is_streaming() {
                    com.send(&[ACK]).await?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }
        }
        Ok(())
    }

    pub async fn cancel(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.recv_state = RecvState::None;
        self.cur_out_file = None;
        self.pending_finished_file = None;
        super::cancel_xymodem_transfer(com).await
    }

    fn finish_received_file(&mut self, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let Some(file) = self.pending_finished_file.take() {
            transfer_state.recieve_state.finish_file(file.keep()?.1);
            self.last_file_finished = true;
        }
        Ok(())
    }

    async fn receive_eot(&mut self, com: &mut dyn Connection, state: &mut TransferState) -> crate::Result<()> {
        let file = self.cur_out_file.take().ok_or(XYModemError::NoFileOpen)?;
        let received = file.as_file().metadata()?.len();
        let actual = if let Some(size) = self.declared_size {
            if received < size {
                self.cancel(com).await?;
                return Err(XYModemError::IncompleteFile(size, received).into());
            }
            truncate_to_file_size(file.path(), size)?
        } else {
            // No wire length in XMODEM: retain the existing CPMEOF heuristic.
            remove_cpm_eof(file.path(), self.last_block_len)?
        };
        state.recieve_state.total_bytes_transfered -= received - actual;
        state.recieve_state.cur_bytes_transfered = actual;
        // Retain RAII ownership until the handshake succeeds. Aborted/incomplete
        // transfers must not leak persisted anonymous files in the temp directory.
        self.pending_finished_file = Some(file);
        if !self.configuration.is_ymodem() {
            com.send(&[ACK]).await?;
            self.finish_received_file(state)?;
            self.recv_state = RecvState::None;
        } else if self.configuration.is_streaming() {
            com.send(&[ACK, self.start_byte()]).await?;
            self.finish_received_file(state)?;
            self.recv_state = RecvState::StartReceive(0);
        } else {
            com.send(&[NAK]).await?;
            self.recv_state = RecvState::ReadBlockStart(1, 0);
        }
        Ok(())
    }

    async fn read_control(&mut self, com: &mut dyn Connection) -> crate::Result<u8> {
        loop {
            let byte = com.read_u8().await?;
            if byte == CAN {
                if self.previous_can {
                    self.cancel(com).await?;
                    return Err(XYModemError::Cancel.into());
                }
                self.previous_can = true;
            } else {
                self.previous_can = false;
                return Ok(byte);
            }
        }
    }

    pub async fn recv(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.await_data(com).await?;
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    async fn await_data(&mut self, com: &mut dyn Connection) -> crate::Result<usize> {
        if self.configuration.is_streaming() {
            com.send(b"G").await?;
        } else if self.configuration.use_crc() {
            com.send(b"C").await?;
        } else {
            com.send(&[NAK]).await?;
        }
        Ok(1)
    }

    /// What the receiver asks the next file with; section 6 wants a G for a streaming batch.
    fn start_byte(&self) -> u8 {
        if self.configuration.is_streaming() {
            b'G'
        } else if self.configuration.use_crc() {
            b'C'
        } else {
            NAK
        }
    }

    fn check_crc(&self, block: &[u8]) -> bool {
        if block.len() < 3 {
            return false;
        }
        match self.configuration.checksum_mode {
            Checksum::Default => {
                let chk = get_checksum(&block[..block.len() - 1]);
                block[block.len() - 1] == chk
            }
            Checksum::CRC16 => {
                let check_crc = get_crc16(&block[..block.len() - 2]);
                let crc = u16::from_be_bytes(block[block.len() - 2..].try_into().unwrap());
                crc == check_crc
            }
        }
    }
}

fn parse_file_info(block: &[u8]) -> Result<(String, Option<u64>), XYModemError> {
    let end = block
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(XYModemError::InvalidFileInfo("missing filename terminator"))?;
    let name = match std::str::from_utf8(&block[..end]) {
        Ok(name) => name.to_owned(),
        Err(_) => str_from_null_terminated_utf8_unchecked(&block[..end]),
    };
    let field = block[end + 1..].split(|byte| *byte == 0 || *byte == b' ').next().unwrap_or_default();
    if field.is_empty() {
        return Ok((name, None));
    }
    let mut size = 0u64;
    for byte in field {
        if !byte.is_ascii_digit() {
            return Err(XYModemError::InvalidFileInfo("invalid decimal size"));
        }
        size = size
            .checked_mul(10)
            .and_then(|size| size.checked_add(u64::from(byte - b'0')))
            .ok_or(XYModemError::InvalidFileInfo("size exceeds u64"))?;
    }
    Ok((name, Some(size)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connection::channel::ChannelConnection, protocol::XYModemVariant};

    #[tokio::test]
    async fn incomplete_and_unconfirmed_files_are_deleted_on_failure() {
        for incomplete in [false, true] {
            let (mut conn, mut peer) = ChannelConnection::create_pair();
            let mut ry = Ry::new(XYModemConfiguration::new(XYModemVariant::YModem));
            let mut state = TransferState::new("cleanup".into());
            let file = NamedTempFile::new().unwrap();
            let path = file.path().to_path_buf();
            ry.cur_out_file = Some(file);
            ry.declared_size = Some(u64::from(incomplete));
            ry.recv_state = RecvState::ReadBlockStart(0, 0);
            peer.send(&[EOT]).await.unwrap();
            let result = ry.update_transfer(&mut conn, &mut state).await;
            if incomplete {
                assert!(result.is_err());
            } else {
                result.unwrap();
                assert!(path.exists(), "awaiting second EOT keeps the file temporary");
                peer.send(&[CAN, CAN]).await.unwrap();
                assert!(ry.update_transfer(&mut conn, &mut state).await.is_err());
            }
            assert!(!path.exists(), "failed receive leaked a temp file");
            assert!(state.recieve_state.finished_files.is_empty());
            assert!(ry.is_finished());
        }
    }
}
