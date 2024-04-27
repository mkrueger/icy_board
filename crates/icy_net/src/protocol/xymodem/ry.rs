use std::io::Write;

use tempfile::NamedTempFile;

use super::{constants::DEFAULT_BLOCK_LENGTH, err::XYModemError, get_checksum, remove_cpm_eof, Checksum, XYModemConfiguration};
use crate::{
    crc::get_crc16,
    protocol::{
        str_from_null_terminated_utf8_unchecked,
        xymodem::constants::{ACK, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
        TransferState,
    },
    Connection,
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

    pub fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        transfer_state.update_time();
        let transfer_info = &mut transfer_state.recieve_state;
        transfer_info.errors = self.errors;
        transfer_info.check_size = self.configuration.get_check_and_size();
        transfer_info.update_bps();
        match self.recv_state {
            RecvState::None => {}

            RecvState::StartReceive(retries) => {
                transfer_state.current_state = "Start receiving...";
                let start = com.read_u8()?;
                if start == SOH {
                    if self.configuration.is_ymodem() {
                        self.recv_state = RecvState::ReadYModemHeader(0);
                    } else {
                        self.cur_out_file = Some(NamedTempFile::new()?);
                        transfer_state.recieve_state.file_name = String::new();
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    }
                } else if start == STX {
                    self.cur_out_file = Some(NamedTempFile::new()?);
                    transfer_state.recieve_state.file_name = String::new();
                    self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                } else {
                    if retries < 3 {
                        self.await_data(com)?;
                    } else if retries == 4 {
                        com.write_all(&[NAK])?;
                    } else {
                        self.cancel(com)?;
                        return Err(XYModemError::TooManyRetriesStarting.into());
                    }
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(retries + 1);
                }
            }

            RecvState::ReadYModemHeader(retries) => {
                let len = 128; // constant header length

                transfer_state.current_state = "Get header...";
                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                let mut block = vec![0; 2 + len + chksum_size];
                com.read_exact(&mut block)?;
                if block[0] != block[1] ^ 0xFF {
                    com.write_all(&[NAK])?;
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(0);
                    return Ok(());
                }
                let block = &block[2..];
                if !self.check_crc(block) {
                    //println!("NAK CRC FAIL");
                    self.errors += 1;
                    com.write_all(&[NAK])?;
                    self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                    return Ok(());
                }
                if block[0] == 0 {
                    // END transfer
                    //println!("END TRANSFER");
                    com.write_all(&[ACK])?;
                    self.recv_state = RecvState::None;
                    return Ok(());
                }

                let file_name = str_from_null_terminated_utf8_unchecked(block);

                let num = str_from_null_terminated_utf8_unchecked(&block[(file_name.len() + 1)..]).to_string();
                let file_size = if let Ok(file_size) = num.parse::<u64>() { file_size } else { 0 };
                transfer_state.recieve_state.file_name = file_name;
                transfer_state.recieve_state.file_size = file_size;
                self.cur_out_file = Some(NamedTempFile::new()?);

                if self.configuration.is_ymodem() {
                    com.write_all(&[ACK, b'C'])?;
                } else {
                    com.write_all(&[ACK])?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }

            RecvState::ReadBlockStart(step, retries) => {
                if step == 0 {
                    let start = com.read_u8()?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                    } else if start == EOT {
                        if let Some(named_file) = self.cur_out_file.take() {
                            let path = &named_file.keep()?.1;
                            remove_cpm_eof(path, self.last_block_len)?;
                            transfer_state.recieve_state.finish_file(path.clone());
                        } else {
                            return Err(XYModemError::NoFileOpen.into());
                        }

                        let transfer_info = &mut transfer_state.recieve_state;
                        transfer_info.log_info("File transferred.");

                        if self.configuration.is_ymodem() {
                            com.write_all(&[NAK])?;
                            self.recv_state = RecvState::ReadBlockStart(1, 0);
                        } else {
                            com.write_all(&[ACK])?;
                            self.recv_state = RecvState::None;
                            transfer_state.is_finished = true;
                        }
                    } else {
                        if retries < 5 {
                            com.write_all(&[NAK])?;
                        } else {
                            self.cancel(com)?;
                            return Err(XYModemError::TooManyRetriesReadingBlock.into());
                        }
                        self.errors += 1;
                        self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    }
                } else if step == 1 {
                    let eot = com.read_u8()?;
                    if eot != EOT {
                        self.recv_state = RecvState::None;
                        return Ok(());
                    }
                    if self.configuration.is_ymodem() {
                        com.write_all(&[ACK, b'C'])?;
                    } else {
                        com.write_all(&[ACK])?;
                    }
                    self.recv_state = RecvState::StartReceive(retries);
                }
            }

            RecvState::ReadBlock(len, retries) => {
                transfer_state.current_state = "Receiving data...";
                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                let mut block = vec![0; 2 + len + chksum_size];
                com.read_exact(&mut block)?;

                if block[0] != block[1] ^ 0xFF {
                    com.write_all(&[NAK])?;

                    self.errors += 1;
                    let start = com.read_u8()?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    } else {
                        self.cancel(com)?;
                        return Err(XYModemError::TooManyRetriesReadingBlock.into());
                    }
                    self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    return Ok(());
                }
                let block = &block[2..];
                if !self.check_crc(block) {
                    //println!("\t\t\t\t\t\trecv crc mismatch");
                    self.errors += 1;
                    com.write_all(&[NAK])?;
                    self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    return Ok(());
                }

                self.last_block_len = len;
                if let Some(named_file) = &mut self.cur_out_file {
                    named_file.as_file_mut().write_all(&block[0..len])?;
                    transfer_state.recieve_state.total_bytes_transfered += len as u64;
                    transfer_state.recieve_state.cur_bytes_transfered += len as u64;
                } else {
                    return Err(XYModemError::NoFileOpen.into());
                }

                if !self.configuration.is_streaming() {
                    com.write_all(&[ACK])?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }
        }
        Ok(())
    }

    pub fn cancel(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.recv_state = RecvState::None;
        super::cancel(com)
    }

    pub fn recv(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.await_data(com)?;
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    fn await_data(&mut self, com: &mut dyn Connection) -> crate::Result<usize> {
        if self.configuration.is_streaming() {
            com.write_all(&[b'G'])?;
        } else if self.configuration.use_crc() {
            com.write_all(&[b'C'])?;
        } else {
            com.write_all(&[NAK])?;
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
    /*
    fn print_block(&self, block: &[u8]) {
        if block[0] == block[1] ^ 0xFF {
            print!("{:02X} {:02X}", block[0], block[1]);
        } else {
            println!("ERR  ERR");
            return;
        }
        let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode {
            2
        } else {
            1
        };
        print!(" Data[{}] ", block.len() - 2 - chksum_size);

        if self.check_crc(&block[2..]) {
            println!("CRC OK ");
        } else {
            println!("CRC ERR");
        }
    }*/
}
