use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use super::{
    constants::{CAN, DEFAULT_BLOCK_LENGTH},
    err::XYModemError,
    get_checksum, Checksum, XYModemConfiguration, XYModemVariant,
};

use crate::{
    crc::get_crc16,
    protocol::{
        xymodem::constants::{ACK, CPMEOF, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
        TransferState,
    },
    Connection,
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

    pub fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        transfer_state.update_time();
        let transfer_info = &mut transfer_state.send_state;
        transfer_info.check_size = self.configuration.get_check_and_size();
        transfer_info.update_bps();

        match self.send_state {
            SendState::None => {}
            SendState::InitiateSend => {
                transfer_state.current_state = "Initiate send…";
                self.get_mode(com)?;
                if self.configuration.is_ymodem() {
                    self.send_state = SendState::SendYModemHeader(0);
                } else {
                    if let Some(next_file) = self.file_queue.pop_front() {
                        transfer_state.send_state.file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
                        transfer_state.send_state.file_size = next_file.metadata()?.len();

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
                    self.cancel(com)?;
                    return Ok(());
                }
                self.block_number = 0;
                //transfer_info.write("Send header...".to_string());
                self.send_ymodem_header(com, transfer_state)?;
                self.send_state = SendState::AckSendYmodemHeader(retries);
            }

            SendState::AckSendYmodemHeader(retries) => {
                // let now = Instant::now();
                let ack = self.read_command(com)?;
                if ack == NAK {
                    transfer_state.current_state = "Encountered error";
                    transfer_state.send_state.errors += 1;
                    if retries > 5 {
                        self.send_state = SendState::None;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    self.send_state = SendState::SendYModemHeader(retries + 1);
                    return Ok(());
                }
                if ack == ACK {
                    if self.transfer_stopped {
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                    transfer_state.current_state = "Header accepted.";
                    let _ = self.read_command(com)?;
                    // SKIP - not needed to check that
                    self.send_state = SendState::SendData(0);
                }

                /*  if now
                    .duration_since(self.last_header_send)
                    .unwrap()
                    .as_millis()
                    > 3000
                {
                    self.send_state = SendState::SendYModemHeader(retries + 1);
                }*/
            }
            SendState::SendData(retries) => {
                transfer_state.current_state = "Send data...";
                if self.send_data_block(com, transfer_state)? {
                    if self.configuration.is_streaming() {
                        self.send_state = SendState::SendData(0);
                        self.check_eof(com, transfer_state)?;
                    } else {
                        self.send_state = SendState::AckSendData(retries);
                    }
                } else {
                    self.send_state = SendState::None;
                };
            }
            SendState::AckSendData(retries) => {
                let ack = self.read_command(com)?;
                if ack == CAN {
                    // need 2 CAN
                    let can2 = self.read_command(com)?;
                    if can2 == CAN {
                        self.send_state = SendState::None;
                        //transfer_info.write("Got cancel ...".to_string());
                        return Err(XYModemError::Cancel.into());
                    }
                }

                if ack != ACK {
                    transfer_state.send_state.errors += 1;

                    // fall back to short block length after too many errors
                    if retries > 3 && self.configuration.block_length == EXT_BLOCK_LENGTH {
                        self.configuration.block_length = DEFAULT_BLOCK_LENGTH;
                        self.send_state = SendState::SendData(retries + 2);
                        return Ok(());
                    }

                    if retries > 5 {
                        self.eot(com)?;
                        return Err(XYModemError::TooManyRetriesSendingHeader.into());
                    }
                    self.send_state = SendState::SendData(retries + 1);
                    return Ok(());
                }
                self.send_state = SendState::SendData(0);
                self.check_eof(com, transfer_state)?;
            }
            SendState::YModemEndHeader(step) => match step {
                0 => {
                    let read_command = self.read_command(com)?;
                    if read_command == NAK {
                        com.write_all(&[EOT])?;
                        self.send_state = SendState::YModemEndHeader(1);
                        return Ok(());
                    }
                    if read_command == ACK {
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                    self.cancel(com)?;
                }
                1 => {
                    if self.read_command(com)? == ACK {
                        self.send_state = SendState::YModemEndHeader(2);
                        return Ok(());
                    }
                    self.cancel(com)?;
                }
                2 => {
                    if self.read_command(com)? == b'C' {
                        self.send_state = SendState::SendYModemHeader(0);
                        return Ok(());
                    }
                    self.cancel(com)?;
                }
                _ => {
                    self.send_state = SendState::None;
                }
            },
        }
        Ok(())
    }

    fn check_eof(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if transfer_state.send_state.cur_bytes_transfered >= transfer_state.send_state.file_size {
            transfer_state.send_state.finish_file(self.cur_file.clone());

            self.eot(com)?;
            if self.configuration.is_ymodem() {
                self.send_state = SendState::YModemEndHeader(0);
            } else {
                self.send_state = SendState::None;
            }
        }
        Ok(())
    }

    #[allow(clippy::unused_self)]
    fn read_command(&self, com: &mut dyn Connection) -> crate::Result<u8> {
        let ch = com.read_u8()?;
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
    fn eot(&self, com: &mut dyn Connection) -> crate::Result<usize> {
        // println!("[EOT]");
        com.write_all(&[EOT])?;
        self.read_command(com)?; // read ACK

        Ok(1)
    }

    pub fn get_mode(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        let ch = self.read_command(com)?;
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

    fn send_block(&mut self, com: &mut dyn Connection, data: &[u8], pad_byte: u8) -> crate::Result<()> {
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
        com.write_all(&block)?;
        self.block_number = self.block_number.wrapping_add(1);
        Ok(())
    }

    fn send_ymodem_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let Some(next_file) = self.file_queue.pop_front() {
            // restart from 0
            let mut block = Vec::new();
            let name = next_file.file_name().unwrap().as_encoded_bytes();
            block.extend_from_slice(name);
            block.push(0);
            let size = next_file.metadata()?.len();
            block.extend_from_slice(format!("{}", size).as_bytes());
            transfer_state.send_state.file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
            transfer_state.send_state.file_size = size;
            transfer_state.send_state.cur_bytes_transfered = 0;

            self.cur_file = next_file.clone();
            let reader = BufReader::new(File::open(next_file)?);
            self.cur_buf = Some(reader);
            self.send_block(com, &block, 0)?;
            Ok(())
        } else {
            self.end_ymodem(com)?;
            Ok(())
        }
    }

    fn send_data_block(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<bool> {
        if let Some(cur) = &mut self.cur_buf {
            let mut block = vec![CPMEOF; self.configuration.block_length];
            let bytes = cur.read(&mut block)?;
            self.send_block(com, &block[0..bytes], CPMEOF)?;
            transfer_state.send_state.total_bytes_transfered += bytes as u64;
            transfer_state.send_state.cur_bytes_transfered += bytes as u64;

            Ok(true)
        } else {
            Err(XYModemError::NoFileOpen.into())
        }
    }

    pub fn cancel(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.send_state = SendState::None;
        super::cancel(com)
    }

    pub fn send(&mut self, files: &[PathBuf]) {
        self.send_state = SendState::InitiateSend;
        for f in files {
            self.file_queue.push_back(f.clone());
        }
    }

    pub fn end_ymodem(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.send_block(com, &[0], 0)?;
        self.transfer_stopped = true;
        Ok(())
    }
}
