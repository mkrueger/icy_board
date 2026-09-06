use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
    time::Duration,
};

use tokio::time::timeout;

use super::{
    Checksum, XYModemConfiguration, XYModemVariant,
    constants::{CAN, DEFAULT_BLOCK_LENGTH},
    err::XYModemError,
    get_checksum,
};

use crate::{
    Connection,
    crc::get_crc16,
    protocol::{
        TransferState,
        xymodem::constants::{ACK, CPMEOF, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
    },
};

/// Timeout for waiting for a response from the receiver (3 seconds)
const READ_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_RETRIES: usize = 5;
// The YMODEM reference specifies up to ten EOT transmissions until ACK.
const MAX_EOT_ATTEMPTS: usize = 10;

#[derive(Debug)]
pub enum SendState {
    None,
    InitiateSend,
    SendYModemHeader(usize),
    AckSendYmodemHeader(usize),
    WaitYModemDataRequest(usize),
    SendData(usize),
    AckSendData(usize),
    YModemWaitNextRequest(usize),
}

pub struct Sy {
    configuration: XYModemConfiguration,

    pub file_queue: VecDeque<PathBuf>,

    block_number: u8,
    send_state: SendState,

    cur_buf: Option<BufReader<File>>,
    cur_file: PathBuf,
    pending_block: Option<Vec<u8>>,
    transfer_stopped: bool,
    previous_can: bool,
}

impl Sy {
    pub fn new(configuration: XYModemConfiguration) -> Self {
        Self {
            configuration,

            cur_file: PathBuf::new(),
            send_state: SendState::None,
            file_queue: VecDeque::new(),
            block_number: match configuration.variant {
                XYModemVariant::YModem | XYModemVariant::YModemG => 0,
                _ => 1,
            },
            transfer_stopped: false,
            cur_buf: None,
            pending_block: None,
            previous_can: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.send_state, SendState::None)
    }

    pub async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        let result = self.update_transfer_inner(com, transfer_state).await;
        if result.is_err() {
            // An I/O failure or cancellation must not leave a resumable state
            // which could later report the current file as successfully sent.
            self.send_state = SendState::None;
            self.cur_buf = None;
            self.pending_block = None;
            transfer_state.is_finished = true;
        }
        result
    }

    async fn update_transfer_inner(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        {
            let transfer_info = &mut transfer_state.send_state;
            transfer_info.check_size = self.configuration.get_check_and_size();
        }
        match self.send_state {
            SendState::None => {}
            SendState::InitiateSend => {
                transfer_state.current_state = "Initiate send…";
                transfer_state.send_state.log_info("Starting transfer, waiting for receiver ready signal...");

                match self.get_mode(com).await {
                    Ok(_) => {
                        let mode_str = match self.configuration.checksum_mode {
                            Checksum::Default => "checksum",
                            Checksum::CRC16 => "CRC16",
                        };
                        let variant_str = format!("{:?}", self.configuration.variant);
                        transfer_state
                            .send_state
                            .log_info(format!("Receiver ready - using {} mode with {} verification", variant_str, mode_str));
                    }
                    Err(e) => {
                        transfer_state.send_state.log_error(format!("Failed to establish connection mode: {}", e));
                        return Err(e);
                    }
                }

                if self.configuration.is_ymodem() {
                    transfer_state.send_state.log_info("Starting YModem batch transfer");
                    self.send_state = SendState::SendYModemHeader(0);
                } else {
                    if let Some(next_file) = self.file_queue.pop_front() {
                        let file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
                        let file_size = next_file.metadata()?.len();
                        transfer_state
                            .send_state
                            .log_info(format!("Starting transfer of '{}' ({} bytes)", file_name, file_size));

                        transfer_state.send_state.file_name = file_name;
                        transfer_state.send_state.file_size = file_size;

                        self.cur_file = next_file.clone();
                        let reader = BufReader::new(File::open(next_file)?);
                        self.cur_buf = Some(reader);
                    }
                    assert!(self.file_queue.is_empty());
                    self.send_state = SendState::SendData(0);
                }
            }

            SendState::SendYModemHeader(retries) => {
                if retries > 0 {
                    transfer_state
                        .send_state
                        .log_info(format!("Retrying YModem header transmission (attempt {})", retries + 1));
                }
                if retries == 0 {
                    self.send_ymodem_header(com, transfer_state).await?;
                } else {
                    self.resend_pending_block(com).await?;
                }
                self.send_state = SendState::AckSendYmodemHeader(retries);
            }

            SendState::AckSendYmodemHeader(retries) => {
                let Some(ack) = self.block_response(com, transfer_state, retries, true).await? else {
                    return Ok(());
                };
                if ack == ACK || self.configuration.is_streaming() && ack == b'G' {
                    if self.transfer_stopped {
                        if ack != ACK {
                            return Err(XYModemError::InvalidResponse(ack).into());
                        }
                        transfer_state.send_state.log_info("Transfer complete - end of batch acknowledged");
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                    transfer_state.current_state = "Header accepted.";
                    transfer_state
                        .send_state
                        .log_info(format!("File header accepted for '{}'", transfer_state.send_state.file_name));
                    if ack == b'G' {
                        self.send_state = SendState::SendData(0);
                    } else {
                        self.wait_data_request(com, transfer_state, 0).await?;
                    }
                } else if ack == CAN {
                    transfer_state.send_state.log_warning("Transfer cancelled by receiver");
                    self.cancel(com).await?;
                    return Err(XYModemError::Cancel.into());
                } else {
                    self.retry_block(com, transfer_state, retries, true).await?;
                }
            }

            SendState::WaitYModemDataRequest(retries) => {
                self.wait_data_request(com, transfer_state, retries).await?;
            }

            SendState::SendData(retries) => {
                transfer_state.current_state = "Send data...";
                if self.configuration.is_streaming() {
                    self.poll_streaming_cancel(com).await?;
                }
                if retries > 0 {
                    transfer_state
                        .send_state
                        .log_warning(format!("Retransmitting block {} (attempt {})", self.block_number, retries + 1));
                }

                let send_result = if retries == 0 {
                    self.send_data_block(com, transfer_state).await
                } else {
                    self.resend_pending_block(com).await.map(|()| true)
                };
                match send_result {
                    Ok(true) => {
                        if self.configuration.is_streaming() {
                            self.send_state = SendState::SendData(0);
                            self.check_eof(com, transfer_state).await?;
                        } else {
                            self.send_state = SendState::AckSendData(retries);
                        }
                    }
                    Ok(false) => {
                        transfer_state.send_state.log_info("End of file reached");
                        // check_eof will perform EOT sequence and transition
                        self.check_eof(com, transfer_state).await?;
                    }
                    Err(e) => {
                        transfer_state.send_state.log_error(format!("Error sending data block: {}", e));
                        return Err(e);
                    }
                }
            }

            SendState::AckSendData(retries) => {
                let Some(ack) = self.block_response(com, transfer_state, retries, false).await? else {
                    return Ok(());
                };
                if ack == CAN {
                    transfer_state.send_state.log_warning("Transfer cancelled by receiver (double CAN)");
                    self.cancel(com).await?;
                    return Err(XYModemError::Cancel.into());
                }

                if ack != ACK {
                    self.retry_block(com, transfer_state, retries, false).await?;
                    return Ok(());
                }

                // ACK ok
                self.send_state = SendState::SendData(0);
                self.check_eof(com, transfer_state).await?;
            }

            SendState::YModemWaitNextRequest(retries) => {
                transfer_state.current_state = "Await next file request";
                self.send_state = if self.wait_request(com, transfer_state, retries, true).await? {
                    SendState::SendYModemHeader(0)
                } else {
                    SendState::YModemWaitNextRequest(retries + 1)
                };
            }
        }
        Ok(())
    }

    async fn check_eof(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if transfer_state.send_state.cur_bytes_transfered >= transfer_state.send_state.file_size {
            transfer_state.send_state.log_info(format!(
                "All bytes sent for '{}' ({} bytes); awaiting EOT acknowledgement",
                transfer_state.send_state.file_name, transfer_state.send_state.cur_bytes_transfered
            ));
            self.eot(com, transfer_state).await?;
            transfer_state.send_state.finish_file(self.cur_file.clone());
            self.cur_buf = None;
            self.pending_block = None;

            if self.configuration.is_ymodem() {
                if !self.file_queue.is_empty() {
                    transfer_state.send_state.log_info(format!("{} file(s) remaining", self.file_queue.len()));
                } else {
                    transfer_state.send_state.log_info("All files sent; terminal header pending");
                }
                self.send_state = SendState::YModemWaitNextRequest(0);
            } else {
                transfer_state.send_state.log_info("XModem transfer complete");
                self.send_state = SendState::None;
            }
        }
        Ok(())
    }

    fn is_timeout(error: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
        matches!(error.downcast_ref::<XYModemError>(), Some(XYModemError::Timeout))
    }

    async fn retry_block(&mut self, com: &mut dyn Connection, state: &mut TransferState, retries: usize, header: bool) -> crate::Result<()> {
        // log_error increments errors; retransmission itself must not count again.
        state
            .send_state
            .log_error(format!("No valid acknowledgement for block {}", self.block_number.wrapping_sub(1)));
        if retries >= MAX_RETRIES {
            self.cancel(com).await?;
            return Err(XYModemError::TooManyRetriesSendingHeader.into());
        }
        self.send_state = if header {
            SendState::SendYModemHeader(retries + 1)
        } else {
            SendState::SendData(retries + 1)
        };
        Ok(())
    }

    async fn block_response(&mut self, com: &mut dyn Connection, state: &mut TransferState, retries: usize, header: bool) -> crate::Result<Option<u8>> {
        match self.read_command(com).await {
            Ok(byte) => Ok(Some(byte)),
            Err(error) if Self::is_timeout(error.as_ref()) => {
                self.retry_block(com, state, retries, header).await?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn request_byte(&self) -> u8 {
        if self.configuration.is_streaming() {
            b'G'
        } else if self.configuration.use_crc() {
            b'C'
        } else {
            NAK
        }
    }

    async fn wait_data_request(&mut self, com: &mut dyn Connection, state: &mut TransferState, retries: usize) -> crate::Result<()> {
        self.send_state = if self.wait_request(com, state, retries, false).await? {
            SendState::SendData(0)
        } else {
            SendState::WaitYModemDataRequest(retries + 1)
        };
        Ok(())
    }

    async fn wait_request(&mut self, com: &mut dyn Connection, state: &mut TransferState, retries: usize, after_eot: bool) -> crate::Result<bool> {
        match self.read_command(com).await {
            Ok(byte) if byte == self.request_byte() => return Ok(true),
            Ok(CAN) => {
                self.cancel(com).await?;
                return Err(XYModemError::Cancel.into());
            }
            // A duplicate header/EOT ACK does not authorize sending data.
            Ok(ACK) => {}
            Ok(NAK) if after_eot => {
                // Preserve the legacy re-EOT fallback, but bound it. In checksum
                // mode NAK is the next header request and was handled above.
                if retries < MAX_RETRIES {
                    self.eot(com, state).await?;
                }
            }
            Ok(byte) => {
                self.cancel(com).await?;
                return Err(XYModemError::InvalidResponse(byte).into());
            }
            Err(error) if Self::is_timeout(error.as_ref()) => {}
            Err(error) => return Err(error),
        }
        state.send_state.log_error("Missing next-file/data request");
        if retries >= MAX_RETRIES {
            self.cancel(com).await?;
            return Err(XYModemError::Timeout.into());
        }
        Ok(false)
    }

    // Require consecutive CAN bytes, retaining the first across streaming polls.
    fn is_cancel(&mut self, byte: u8) -> bool {
        let cancelled = self.previous_can && byte == CAN;
        self.previous_can = byte == CAN;
        cancelled
    }

    async fn read_command(&mut self, com: &mut dyn Connection) -> crate::Result<u8> {
        timeout(READ_TIMEOUT, async {
            loop {
                let byte = com.read_u8().await?;
                if self.is_cancel(byte) || byte != CAN {
                    return Ok(byte);
                }
            }
        })
        .await
        .map_err(|_| XYModemError::Timeout)?
    }

    async fn poll_streaming_cancel(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        // Bound each poll even if a noisy peer continuously supplies bytes.
        for _ in 0..64 {
            let mut byte = [0];
            if com.try_read(&mut byte).await? == 0 {
                break;
            }
            if self.is_cancel(byte[0]) {
                self.cancel(com).await?;
                return Err(XYModemError::Cancel.into());
            }
        }
        Ok(())
    }

    async fn eot(&mut self, com: &mut dyn Connection, state: &mut TransferState) -> crate::Result<()> {
        for attempt in 0..MAX_EOT_ATTEMPTS {
            com.send(&[EOT]).await?;
            match self.read_command(com).await {
                Ok(ACK) => return Ok(()),
                Ok(CAN) => {
                    self.cancel(com).await?;
                    return Err(XYModemError::Cancel.into());
                }
                // Classic YMODEM commonly uses EOT/NAK/EOT/ACK. Direct ACK
                // is also valid; XMODEM must retry EOT after NAK or timeout.
                Ok(NAK) if attempt == 0 && self.configuration.is_ymodem() && !self.configuration.is_streaming() => continue,
                Ok(_) => {}
                Err(error) if Self::is_timeout(error.as_ref()) => {}
                Err(error) => return Err(error),
            }
            state.send_state.log_error("EOT not acknowledged");
        }
        self.cancel(com).await?;
        Err(XYModemError::Timeout.into())
    }

    pub async fn get_mode(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        let ch = self.read_command(com).await?;
        match ch {
            NAK => {
                self.configuration.checksum_mode = Checksum::Default;
                self.configuration.streaming_enabled = false;
                Ok(())
            }
            b'C' => {
                self.configuration.checksum_mode = Checksum::CRC16;
                self.configuration.streaming_enabled = false;
                Ok(())
            }
            b'G' => {
                self.configuration = if self.configuration.is_ymodem() {
                    XYModemConfiguration::new(XYModemVariant::YModemG)
                } else {
                    XYModemConfiguration::new(XYModemVariant::XModem1kG)
                };
                Ok(())
            }
            CAN => Err(XYModemError::Cancel.into()),
            _ => Err(XYModemError::InvalidMode(ch).into()),
        }
    }

    async fn send_block(&mut self, com: &mut dyn Connection, data: &[u8], pad_byte: u8) -> crate::Result<()> {
        if data.len() > EXT_BLOCK_LENGTH {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "X/YMODEM block exceeds 1024 bytes").into());
        }
        let block_len = if data.len() <= DEFAULT_BLOCK_LENGTH { SOH } else { STX };
        let mut block = Vec::new();
        block.push(block_len);
        block.push(self.block_number);
        block.push(!self.block_number);
        block.extend_from_slice(data);
        block.resize((if block_len == SOH { DEFAULT_BLOCK_LENGTH } else { EXT_BLOCK_LENGTH }) + 3, pad_byte);

        match self.configuration.checksum_mode {
            Checksum::Default => {
                let chk_sum = get_checksum(&block[3..]);
                block.push(chk_sum);
            }
            Checksum::CRC16 => {
                let crc = get_crc16(&block[3..]);
                block.extend_from_slice(&u16::to_be_bytes(crc));
            }
        }
        // println!("Send block {:X?}", block);
        com.send(&block).await?;
        self.pending_block = Some(block);
        self.block_number = self.block_number.wrapping_add(1);
        Ok(())
    }

    async fn resend_pending_block(&self, com: &mut dyn Connection) -> crate::Result<()> {
        let Some(block) = &self.pending_block else {
            return Err(XYModemError::NoPendingBlock.into());
        };
        com.send(block).await?;
        Ok(())
    }

    async fn send_ymodem_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        // Always reset block number for any header (file or terminal)
        self.block_number = 0;

        if let Some(next_file) = self.file_queue.pop_front() {
            let mut block = Vec::new();
            let name_bytes = next_file.file_name().unwrap().as_encoded_bytes();
            block.extend_from_slice(name_bytes);
            block.push(0);
            let size = next_file.metadata()?.len();
            block.extend_from_slice(format!("{}", size).as_bytes());
            block.push(0);

            let file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
            transfer_state
                .send_state
                .log_info(format!("Sending header for '{}' ({} bytes)", file_name, size));

            transfer_state.send_state.file_name = file_name;
            transfer_state.send_state.file_size = size;
            transfer_state.send_state.cur_bytes_transfered = 0;

            self.cur_file = next_file.clone();
            self.cur_buf = Some(BufReader::new(File::open(next_file)?));

            self.send_block(com, &block, 0).await?;
            Ok(())
        } else {
            transfer_state.send_state.log_info("Sending terminal empty header (batch end)");
            self.end_ymodem(com).await?;
            Ok(())
        }
    }

    async fn send_data_block(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<bool> {
        if let Some(cur) = &mut self.cur_buf {
            let expected = transfer_state.send_state.file_size;
            let sent = transfer_state.send_state.cur_bytes_transfered;
            let remaining = expected.saturating_sub(sent);
            if remaining == 0 {
                return Ok(false);
            }
            // Fill a complete block (or the declared final partial block).
            // Short reads must not insert padding into the middle of a file;
            // premature EOF must fail rather than repeatedly calling check_eof.
            let mut block = vec![CPMEOF; remaining.min(self.configuration.block_length as u64) as usize];
            let mut bytes = 0;
            while bytes < block.len() {
                match cur.read(&mut block[bytes..]) {
                    Ok(0) => {
                        self.cancel(com).await?;
                        return Err(XYModemError::IncompleteFile(expected, sent + bytes as u64).into());
                    }
                    Ok(n) => bytes += n,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error.into()),
                }
            }
            self.send_block(com, &block[0..bytes], CPMEOF).await?;
            transfer_state.send_state.total_bytes_transfered += bytes as u64;
            transfer_state.send_state.cur_bytes_transfered += bytes as u64;

            Ok(true)
        } else {
            Err(XYModemError::NoFileOpen.into())
        }
    }

    pub async fn cancel(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.send_state = SendState::None;
        super::cancel_xymodem_transfer(com).await
    }

    pub fn send(&mut self, files: &[PathBuf]) {
        self.send_state = SendState::InitiateSend;
        for f in files {
            self.file_queue.push_back(f.clone());
        }
    }

    pub async fn end_ymodem(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.send_block(com, &[0], 0).await?;
        self.transfer_stopped = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Linux limits a filename component to 255 bytes, so an integration test
    // cannot construct an oversized header via a real source pathname.
    #[tokio::test]
    async fn oversized_header_is_rejected_before_sending_or_advancing() {
        let mut sender = Sy::new(XYModemConfiguration::new(XYModemVariant::YModem));
        let mut connection = crate::connection::NullConnection {};
        let error = sender.send_block(&mut connection, &[b'a'; EXT_BLOCK_LENGTH + 1], 0).await.unwrap_err();
        assert_eq!(error.downcast_ref::<std::io::Error>().unwrap().kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(sender.block_number, 0);
        assert!(sender.pending_block.is_none());
    }
}
