use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

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

#[derive(Debug)]
pub enum SendState {
    None,
    InitiateSend,
    SendYModemHeader(usize),
    AckSendYmodemHeader(usize),
    SendData(usize),
    AckSendData(usize),
    YModemEndHeader(u8),
    YModemWaitNextRequest,
}

pub struct Sy {
    configuration: XYModemConfiguration,

    pub file_queue: VecDeque<PathBuf>,

    block_number: u8,
    send_state: SendState,

    cur_buf: Option<BufReader<File>>,
    cur_file: PathBuf,
    transfer_stopped: bool,
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
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.send_state, SendState::None)
    }

    pub async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
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
                            .log_info(&format!("Receiver ready - using {} mode with {} verification", variant_str, mode_str));
                    }
                    Err(e) => {
                        transfer_state.send_state.log_error(&format!("Failed to establish connection mode: {}", e));
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
                            .log_info(&format!("Starting transfer of '{}' ({} bytes)", file_name, file_size));

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
                if retries > 3 {
                    transfer_state.current_state = "Too many retries...aborting";
                    transfer_state
                        .send_state
                        .log_error("Maximum retries exceeded while sending YModem header, aborting transfer");
                    self.cancel(com).await?;
                    return Ok(());
                }
                if retries > 0 {
                    transfer_state
                        .send_state
                        .log_info(&format!("Retrying YModem header transmission (attempt {})", retries + 1));
                }
                self.block_number = 0;
                self.send_ymodem_header(com, transfer_state).await?;
                self.send_state = SendState::AckSendYmodemHeader(retries);
            }

            SendState::AckSendYmodemHeader(retries) => {
                let ack = self.read_command(com).await?;
                if ack == NAK {
                    transfer_state.send_state.errors += 1;
                    transfer_state.current_state = "Encountered error";
                    transfer_state
                        .send_state
                        .log_info(&format!("NAK received for YModem header (retry {})", retries + 1));

                    if retries > 5 {
                        transfer_state.send_state.log_error("Too many NAKs for YModem header, aborting");
                        self.send_state = SendState::None;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    self.send_state = SendState::SendYModemHeader(retries + 1);
                    return Ok(());
                } else if ack == ACK {
                    if self.transfer_stopped {
                        transfer_state.send_state.log_info("Transfer complete - end of batch acknowledged");
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                    transfer_state.current_state = "Header accepted.";
                    transfer_state
                        .send_state
                        .log_info(&format!("File header accepted for '{}'", transfer_state.send_state.file_name));
                    let _ = self.read_command(com).await?;
                    // SKIP - not needed to check that
                    self.send_state = SendState::SendData(0);
                } else if ack == CAN {
                    transfer_state.send_state.log_warning("Transfer cancelled by receiver");
                    self.cancel(com).await?;
                    return Err(XYModemError::Cancel.into());
                } else {
                    transfer_state
                        .send_state
                        .log_error(&format!("Unexpected response to YModem header: 0x{:02X}", ack));
                    transfer_state.send_state.errors += 1;
                    self.send_state = SendState::SendYModemHeader(retries + 1);
                }
            }

            SendState::SendData(retries) => {
                transfer_state.current_state = "Send data...";
                if retries > 0 {
                    transfer_state
                        .send_state
                        .log_warning(&format!("Retransmitting block {} (attempt {})", self.block_number, retries + 1));
                }

                match self.send_data_block(com, transfer_state).await {
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
                        transfer_state.send_state.log_error(&format!("Error sending data block: {}", e));
                        return Err(e);
                    }
                }
            }

            SendState::AckSendData(retries) => {
                let ack = self.read_command(com).await?;
                if ack == CAN {
                    let can2 = self.read_command(com).await?;
                    if can2 == CAN {
                        transfer_state.send_state.log_warning("Transfer cancelled by receiver (double CAN)");
                        self.send_state = SendState::None;
                        return Err(XYModemError::Cancel.into());
                    }
                }

                if ack != ACK {
                    transfer_state.send_state.errors += 1;
                    transfer_state.send_state.log_error(&format!(
                        "NAK/error for block {} (error count: {})",
                        self.block_number.wrapping_sub(1),
                        transfer_state.send_state.errors
                    ));

                    if retries > 3 && self.configuration.block_length == EXT_BLOCK_LENGTH {
                        transfer_state.send_state.log_error("Falling back to 128-byte blocks due to errors");
                        self.configuration.block_length = DEFAULT_BLOCK_LENGTH;
                        self.send_state = SendState::SendData(retries + 2);
                        return Ok(());
                    }

                    if retries > 5 {
                        transfer_state.send_state.log_error("Max retries for data block; aborting with cancel");
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    self.send_state = SendState::SendData(retries + 1);
                    return Ok(());
                }

                // ACK ok
                self.send_state = SendState::SendData(0);
                self.check_eof(com, transfer_state).await?;
            }

            SendState::YModemWaitNextRequest => {
                transfer_state.current_state = "Await next file request";
                let cmd = self.read_command(com).await?;
                if cmd == CAN {
                    if self.read_command(com).await? == CAN {
                        transfer_state.send_state.log_warning("Batch cancelled (double CAN)");
                        self.send_state = SendState::None;
                        return Err(XYModemError::Cancel.into());
                    }
                }

                let streaming = self.configuration.variant == XYModemVariant::YModemG;
                let expected = if streaming { b'G' } else { b'C' };

                match cmd {
                    c if c == expected => {
                        if !self.file_queue.is_empty() {
                            transfer_state
                                .send_state
                                .log_info(&format!("Receiver ready for next file ({} remaining)", self.file_queue.len()));
                            self.send_state = SendState::SendYModemHeader(0);
                        } else {
                            transfer_state.send_state.log_info("Receiver requested terminal header; sending empty block");
                            self.send_state = SendState::SendYModemHeader(0);
                        }
                    }
                    ACK => {
                        // Stray ACK – wait again
                        transfer_state.send_state.log_info("Stray ACK while waiting for 'C'/'G'; ignoring");
                        self.send_state = SendState::YModemWaitNextRequest;
                    }
                    NAK => {
                        // Some quirky receivers re-send NAK; re-EOT then wait again
                        transfer_state.send_state.log_warning("Got NAK during next-file wait; re-sending EOT handshake");
                        self.eot(com).await?;
                        self.send_state = SendState::YModemWaitNextRequest;
                    }
                    other => {
                        transfer_state
                            .send_state
                            .log_error(&format!("Unexpected command 0x{:02X} awaiting next file", other));
                        self.cancel(com).await?;
                        return Err(XYModemError::InvalidResponse(other).into());
                    }
                }
            }

            SendState::YModemEndHeader(step) => match step {
                0 => {
                    transfer_state.send_state.log_info("Waiting for next file request or batch end confirmation");
                    let read_command = self.read_command(com).await?;
                    if read_command == NAK {
                        transfer_state.send_state.log_info("Receiver requesting EOT confirmation");
                        com.send(&[EOT]).await?;
                        self.send_state = SendState::YModemEndHeader(1);
                        return Ok(());
                    }
                    if read_command == ACK {
                        transfer_state.send_state.log_info("Batch transfer complete");
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                    transfer_state
                        .send_state
                        .log_warning(&format!("Unexpected response during batch end: 0x{:02X}", read_command));
                    self.cancel(com).await?;
                }
                1 => {
                    if self.read_command(com).await? == ACK {
                        transfer_state.send_state.log_info("EOT acknowledged, waiting for next file request");
                        self.send_state = SendState::YModemEndHeader(2);
                        return Ok(());
                    }
                    transfer_state.send_state.log_error("Failed to receive EOT acknowledgment");
                    self.cancel(com).await?;
                }
                2 => {
                    if self.read_command(com).await? == b'C' {
                        if !self.file_queue.is_empty() {
                            transfer_state
                                .send_state
                                .log_info(&format!("Receiver ready for next file in batch ({} files remaining)", self.file_queue.len()));
                            self.send_state = SendState::SendYModemHeader(0);
                        } else {
                            transfer_state.send_state.log_info("No more files in batch, sending end-of-batch header");
                            self.send_state = SendState::SendYModemHeader(0);
                        }
                        return Ok(());
                    }
                    transfer_state
                        .send_state
                        .log_error("Expected 'C' for next file but received different response");
                    self.cancel(com).await?;
                }
                _ => {
                    self.send_state = SendState::None;
                }
            },
        }
        Ok(())
    }

    async fn check_eof(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if transfer_state.send_state.cur_bytes_transfered >= transfer_state.send_state.file_size {
            transfer_state.send_state.log_info(&format!(
                "File '{}' complete ({} bytes)",
                transfer_state.send_state.file_name, transfer_state.send_state.cur_bytes_transfered
            ));
            transfer_state.send_state.finish_file(self.cur_file.clone());

            self.eot(com).await?;

            if self.configuration.is_ymodem() {
                if !self.file_queue.is_empty() {
                    transfer_state.send_state.log_info(&format!("{} file(s) remaining", self.file_queue.len()));
                } else {
                    transfer_state.send_state.log_info("All files sent; terminal header pending");
                }
                self.send_state = SendState::YModemWaitNextRequest;
            } else {
                transfer_state.send_state.log_info("XModem transfer complete");
                self.send_state = SendState::None;
            }
        }
        Ok(())
    }

    #[allow(clippy::unused_self)]
    async fn read_command(&self, com: &mut dyn Connection) -> crate::Result<u8> {
        let ch = com.read_u8().await?;
        /*
         let cmd = match ch {
            b'C' => "[C]",
            EOT => "[EOT]",
            ACK => "[ACK]",
            NAK => "[NAK]",
            CAN => "[CAN]",
            _ => ""
        };
        println!("GOT CMD: #{} (0x{:X})", cmd, ch);*/

        Ok(ch)
    }

    #[allow(clippy::unused_self)]
    async fn eot(&self, com: &mut dyn Connection) -> crate::Result<()> {
        // First EOT
        com.send(&[EOT]).await?;
        let first = self.read_command(com).await?;

        // Streaming YModemG: expect ACK only, no double handshake
        if self.configuration.variant == XYModemVariant::YModemG {
            if first != ACK {
                return Err(XYModemError::InvalidResponse(first).into());
            }
            return Ok(());
        }

        // Classic YModem: prefer NAK then EOT then ACK
        match first {
            NAK => {
                // Resend EOT
                com.send(&[EOT]).await?;
                let second = self.read_command(com).await?;
                if second != ACK {
                    return Err(XYModemError::InvalidResponse(second).into());
                }
                Ok(())
            }
            ACK => {
                // Lenient receivers ACK immediately – accept
                Ok(())
            }
            other => Err(XYModemError::InvalidResponse(other).into()),
        }
    }

    pub async fn get_mode(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        let ch = self.read_command(com).await?;
        match ch {
            NAK => {
                self.configuration.checksum_mode = Checksum::Default;
                Ok(())
            }
            b'C' => {
                self.configuration.checksum_mode = Checksum::CRC16;
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
        self.block_number = self.block_number.wrapping_add(1);
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

            let file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
            transfer_state
                .send_state
                .log_info(&format!("Sending header for '{}' ({} bytes)", file_name, size));

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
            let mut block = vec![CPMEOF; self.configuration.block_length];
            let bytes = cur.read(&mut block)?;
            if bytes == 0 {
                return Ok(false);
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
