use std::io::Write;

use tempfile::NamedTempFile;

use super::{Checksum, XYModemConfiguration, constants::DEFAULT_BLOCK_LENGTH, err::XYModemError, get_checksum, remove_cpm_eof};
use crate::{
    Connection,
    crc::get_crc16,
    protocol::{
        TransferState, str_from_null_terminated_utf8_unchecked,
        xymodem::constants::{ACK, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
    },
};

#[derive(Debug)]
pub enum RecvState {
    None,

    StartReceive(usize),
    ReadYModemHeader(usize),
    ReadBlock(usize, usize),
    ReadBlockStart(u8, usize),
}

/// specification: <http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt>
pub struct Ry {
    configuration: XYModemConfiguration,
    errors: usize,
    recv_state: RecvState,
    cur_out_file: Option<NamedTempFile>,
    last_block_len: usize,
}

impl Ry {
    pub fn new(configuration: XYModemConfiguration) -> Self {
        Ry {
            configuration,
            recv_state: RecvState::None,
            errors: 0,
            last_block_len: 0,
            cur_out_file: None,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.recv_state, RecvState::None)
    }

    pub async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        transfer_state.update_time();
        {
            let transfer_info = &mut transfer_state.recieve_state;
            transfer_info.errors = self.errors;
            transfer_info.check_size = self.configuration.get_check_and_size();
            transfer_info.update_bps();
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
                        .log_info(&format!("Starting {} receive with {} verification", variant_str, mode_str));
                }

                let start = com.read_u8().await?;
                if start == SOH {
                    transfer_state.recieve_state.log_info("Received SOH - 128 byte blocks");
                    if self.configuration.is_ymodem() {
                        self.recv_state = RecvState::ReadYModemHeader(0);
                    } else {
                        self.cur_out_file = Some(NamedTempFile::new()?);
                        transfer_state.recieve_state.file_name = String::new();
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    }
                } else if start == STX {
                    transfer_state.recieve_state.log_info("Received STX - 1024 byte blocks");
                    self.cur_out_file = Some(NamedTempFile::new()?);
                    transfer_state.recieve_state.file_name = String::new();
                    self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                } else {
                    transfer_state
                        .recieve_state
                        .log_warning(&format!("Invalid start byte: 0x{:02X} (retry {})", start, retries));
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

            RecvState::ReadYModemHeader(retries) => {
                let len = 128; // constant header length

                transfer_state.current_state = "Get header...";
                if retries > 0 {
                    transfer_state
                        .recieve_state
                        .log_warning(&format!("Retrying YModem header read (attempt {})", retries + 1));
                }

                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                let mut block = vec![0; 2 + len + chksum_size];
                com.read_exact(&mut block).await?;

                if block[0] != block[1] ^ 0xFF {
                    transfer_state
                        .recieve_state
                        .log_error(&format!("Block number check failed: {:02X} != {:02X}^FF", block[0], block[1]));
                    com.send(&[NAK]).await?;
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(0);
                    return Ok(());
                }

                let block = &block[2..];
                if !self.check_crc(block) {
                    transfer_state.recieve_state.log_error("YModem header CRC/checksum verification failed");
                    self.errors += 1;
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                    return Ok(());
                }

                if block[0] == 0 {
                    transfer_state.recieve_state.log_info("End of batch transfer detected");
                    com.send(&[ACK]).await?;
                    self.recv_state = RecvState::None;
                    return Ok(());
                }

                let file_name = str_from_null_terminated_utf8_unchecked(block);
                let num = str_from_null_terminated_utf8_unchecked(&block[(file_name.len() + 1)..]).to_string();
                let file_size = if let Ok(file_size) = num.parse::<u64>() {
                    file_size
                } else {
                    transfer_state.recieve_state.log_warning(&format!("Could not parse file size: '{}'", num));
                    0
                };

                transfer_state
                    .recieve_state
                    .log_info(&format!("Receiving file '{}' ({} bytes)", file_name, file_size));
                transfer_state.recieve_state.file_name = file_name;
                transfer_state.recieve_state.file_size = file_size;
                self.cur_out_file = Some(NamedTempFile::new()?);

                if self.configuration.is_ymodem() {
                    transfer_state.recieve_state.log_info("Sending ACK+C for YModem data blocks");
                    com.send(&[ACK, b'C']).await?;
                } else {
                    com.send(&[ACK]).await?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }

            RecvState::ReadBlockStart(step, retries) => {
                if step == 0 {
                    let start = com.read_u8().await?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                    } else if start == EOT {
                        transfer_state.recieve_state.log_info("EOT received - end of file");

                        if let Some(named_file) = self.cur_out_file.take() {
                            let path = &named_file.keep()?.1;
                            remove_cpm_eof(path, self.last_block_len)?;

                            let file_info = if !transfer_state.recieve_state.file_name.is_empty() {
                                format!(
                                    "'{}' ({} bytes)",
                                    transfer_state.recieve_state.file_name, transfer_state.recieve_state.cur_bytes_transfered
                                )
                            } else {
                                format!("{} bytes", transfer_state.recieve_state.cur_bytes_transfered)
                            };
                            transfer_state.recieve_state.log_info(&format!("File transfer complete: {}", file_info));
                            transfer_state.recieve_state.finish_file(path.clone());
                        } else {
                            transfer_state.recieve_state.log_error("No file open when EOT received");
                            return Err(XYModemError::NoFileOpen.into());
                        }

                        if self.configuration.is_ymodem() {
                            transfer_state.recieve_state.log_info("Sending NAK for YModem EOT confirmation");
                            com.send(&[NAK]).await?;
                            self.recv_state = RecvState::ReadBlockStart(1, 0);
                        } else {
                            transfer_state.recieve_state.log_info("XModem transfer complete");
                            com.send(&[ACK]).await?;
                            self.recv_state = RecvState::None;
                            transfer_state.is_finished = true;
                        }
                    } else {
                        transfer_state
                            .recieve_state
                            .log_warning(&format!("Invalid block start byte: 0x{:02X} (retry {})", start, retries));
                        if retries < 5 {
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
                    let eot = com.read_u8().await?;
                    if eot != EOT {
                        transfer_state
                            .recieve_state
                            .log_warning(&format!("Expected second EOT but received: 0x{:02X}", eot));
                        self.recv_state = RecvState::None;
                        return Ok(());
                    }
                    transfer_state.recieve_state.log_info("Second EOT confirmed");

                    if self.configuration.is_ymodem() {
                        transfer_state.recieve_state.log_info("Ready for next file in batch");
                        com.send(&[ACK, b'C']).await?;
                    } else {
                        com.send(&[ACK]).await?;
                    }
                    self.recv_state = RecvState::StartReceive(retries);
                }
            }

            RecvState::ReadBlock(len, retries) => {
                transfer_state.current_state = "Receiving data...";
                if retries > 0 {
                    transfer_state
                        .recieve_state
                        .log_warning(&format!("Retrying block read (attempt {}, {} bytes)", retries + 1, len));
                }

                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                let mut block = vec![0; 2 + len + chksum_size];
                com.read_exact(&mut block).await?;

                let block_num = block[0];
                let block_num_inv = block[1];

                if block_num != block_num_inv ^ 0xFF {
                    transfer_state.recieve_state.log_error(&format!(
                        "Block number verification failed: {:02X} != {:02X}^FF (block {})",
                        block_num, block_num_inv, block_num
                    ));
                    com.send(&[NAK]).await?;
                    self.errors += 1;

                    let start = com.read_u8().await?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    } else {
                        transfer_state.recieve_state.log_error("Failed to recover after block number error");
                        self.cancel(com).await?;
                        return Err(XYModemError::TooManyRetriesReadingBlock.into());
                    }
                    return Ok(());
                }

                let block = &block[2..];
                if !self.check_crc(block) {
                    transfer_state.recieve_state.log_error(&format!(
                        "CRC/checksum verification failed for block {} (error count: {})",
                        block_num,
                        self.errors + 1
                    ));
                    self.errors += 1;
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    return Ok(());
                }

                self.last_block_len = len;
                if let Some(named_file) = &mut self.cur_out_file {
                    named_file.as_file_mut().write_all(&block[0..len])?;
                    transfer_state.recieve_state.total_bytes_transfered += len as u64;
                    transfer_state.recieve_state.cur_bytes_transfered += len as u64;
                } else {
                    transfer_state.recieve_state.log_error("No file open for writing block data");
                    return Err(XYModemError::NoFileOpen.into());
                }

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
        super::cancel_xymodem_transfer(com).await
    }

    pub async fn recv(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.await_data(com).await?;
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    async fn await_data(&mut self, com: &mut dyn Connection) -> crate::Result<usize> {
        if self.configuration.is_streaming() {
            com.send(&[b'G']).await?;
        } else if self.configuration.use_crc() {
            com.send(&[b'C']).await?;
        } else {
            com.send(&[NAK]).await?;
        }
        Ok(1)
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
