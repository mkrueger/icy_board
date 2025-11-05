#![allow(clippy::unused_self)]

use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read, Seek},
    path::PathBuf,
};

use crate::{
    Connection,
    protocol::{Header, HeaderType, TransferState, ZCRCE, ZCRCG, ZFrameType, Zmodem, zfile_flag, zmodem::err::ZModemError},
};

use super::{ZCRCQ, ZCRCW};

#[derive(Debug, PartialEq)]
pub enum SendState {
    Await,
    SendZRQInit,
    SendZDATA,
    SendDataPackages,
    SendNextFile,
}

pub struct Sz {
    state: SendState,
    pub file_queue: VecDeque<PathBuf>,
    cur_buf: Option<BufReader<File>>,
    cur_file: PathBuf,

    pub errors: usize,
    pub package_len: usize,
    pub transfered_file: bool,

    retries: usize,
    can_count: usize,
    receiver_capabilities: u8,
    nonstop: bool,
}

impl Sz {
    pub fn new(block_length: usize) -> Self {
        Self {
            state: SendState::Await,
            file_queue: VecDeque::new(),
            transfered_file: false,
            cur_buf: None,
            cur_file: PathBuf::new(),
            errors: 0,
            retries: 0,
            receiver_capabilities: 0,
            can_count: 0,
            package_len: block_length,
            nonstop: true,
        }
    }

    fn can_fdx(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::CANFDX != 0
    }
    fn can_receive_data_during_io(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::CANOVIO != 0
    }
    fn _can_send_break(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::CANBRK != 0
    }
    fn _can_decrypt(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::CANCRY != 0
    }
    fn _can_lzw(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::CANLZW != 0
    }
    fn can_use_crc32(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::CANFC32 != 0
    }
    fn can_esc_control(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::ESCCTL != 0
    }
    fn _can_esc_8thbit(&self) -> bool {
        self.receiver_capabilities & super::zrinit_flag::ESC8 != 0
    }

    fn get_header_type(&self) -> HeaderType {
        if self.can_use_crc32() { HeaderType::Bin32 } else { HeaderType::Bin }
    }

    fn encode_subpacket(&self, zcrc_byte: u8, data: &[u8]) -> Vec<u8> {
        if self.can_use_crc32() {
            Zmodem::encode_subpacket_crc32(zcrc_byte, data, self.can_esc_control())
        } else {
            Zmodem::encode_subpacket_crc16(zcrc_byte, data, self.can_esc_control())
        }
    }
    fn next_file(&mut self, transfer_state: &mut TransferState) -> crate::Result<bool> {
        if let Some(next_file) = self.file_queue.pop_front() {
            transfer_state.send_state.file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
            transfer_state.send_state.file_size = next_file.metadata()?.len();

            transfer_state.send_state.log_info(format!(
                "Starting file: {} ({} bytes, {} files remaining)",
                transfer_state.send_state.file_name,
                transfer_state.send_state.file_size,
                self.file_queue.len()
            ));

            self.cur_file = next_file.clone();
            let reader = BufReader::new(File::open(next_file)?);
            self.cur_buf = Some(reader);
            Ok(true)
        } else {
            transfer_state.send_state.log_info("No more files to send".to_string());
            Ok(false)
        }
    }

    pub async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if transfer_state.is_finished {
            return Ok(());
        }
        if self.retries > 5 {
            transfer_state.send_state.log_error("Too many retries, cancelling transfer".to_string());
            Zmodem::cancel(com).await?;
            transfer_state.is_finished = true;
            return Ok(());
        }
        transfer_state.update_time();
        let transfer_info = &mut transfer_state.send_state;
        transfer_info.errors = self.errors;
        transfer_info.check_size = format!("Crc32/{}", self.package_len);
        transfer_info.update_bps();

        if let Some(header) = Header::try_read(com, &mut self.can_count).await? {
            println!("found out of order header: {:?}", header);
            return self.handle_header(com, transfer_state, header).await;
        }

        match self.state {
            SendState::Await => {
                self.read_next_header(com, transfer_state).await?;
            }

            SendState::SendNextFile => {
                self.state = SendState::Await;
                if !self.next_file(transfer_state)? {
                    transfer_state.send_state.log_info("All files sent, sending ZFIN".to_string());
                    self.send_zfin(com, transfer_state.send_state.cur_bytes_transfered as u32).await?;
                    transfer_state.send_state.cur_bytes_transfered = 0;
                    return Ok(());
                }
                self.send_zfile(com, transfer_state, 0).await?;
            }

            SendState::SendZRQInit => {
                transfer_state.send_state.log_info(format!("Sending ZRQINIT (attempt {}/5)", self.retries + 1));
                self.send_zrqinit(com).await?;
                self.state = SendState::Await;
                self.retries += 1;
            }
            SendState::SendZDATA => {
                if self.cur_buf.is_none() {
                    transfer_state.send_state.log_error("No file buffer available for ZDATA".to_string());
                    transfer_state.is_finished = true;
                    return Ok(());
                }

                transfer_state
                    .send_state
                    .log_info(format!("Sending ZDATA header at offset {}", transfer_state.send_state.cur_bytes_transfered));

                Header::from_number(ZFrameType::Data, transfer_state.send_state.cur_bytes_transfered as u32)
                    .write(com, self.get_header_type(), self.can_esc_control())
                    .await?;
                self.state = SendState::SendDataPackages;
            }
            SendState::SendDataPackages => {
                let mut p = Vec::new();
                if self.cur_buf.is_none() {
                    return Ok(());
                }
                let old_pos = transfer_state.send_state.cur_bytes_transfered;

                // Determine if this is the last block
                let remaining = transfer_state.send_state.file_size - transfer_state.send_state.cur_bytes_transfered;
                let is_last_block = remaining as usize <= self.package_len;

                // Fixed CRC byte selection according to spec
                let crc_byte = if remaining == 0 {
                    ZCRCW // No more data to send, ACK expected
                } else if is_last_block {
                    // Last data subpacket of file
                    ZCRCE // End of frame, no ACK expected
                } else {
                    // More data to follow
                    if self.nonstop {
                        ZCRCG // Frame continues nonstop
                    } else {
                        ZCRCQ // Frame continues, ZACK expected
                    }
                };

                if let Some(cur) = &mut self.cur_buf {
                    let mut block = vec![0; self.package_len];
                    let bytes = cur.read(&mut block)?;
                    p.extend_from_slice(&self.encode_subpacket(crc_byte, &block[..bytes]));
                    transfer_state.send_state.total_bytes_transfered += bytes as u64;
                    transfer_state.send_state.cur_bytes_transfered += bytes as u64;
                }

                com.send(&p).await?;

                // Handle end of file
                if transfer_state.send_state.cur_bytes_transfered >= transfer_state.send_state.file_size {
                    transfer_state.send_state.log_info(format!(
                        "File complete, sending ZEOF ({})",
                        if self.nonstop { "streaming mode" } else { "waiting for ACK" }
                    ));

                    if crc_byte == ZCRCW {
                        let ack = Header::read(com, &mut self.can_count).await?;
                        if let Some(header) = ack {
                            match header.frame_type {
                                ZFrameType::Ack => {
                                    transfer_state.send_state.log_info("Received ACK for final data".to_string());
                                }
                                ZFrameType::RPos => {
                                    // Reposition and continue
                                    let new_pos = header.number() as u64;
                                    transfer_state.send_state.log_warning(format!(
                                        "Receiver requested reposition to {} (was at {})",
                                        new_pos, transfer_state.send_state.cur_bytes_transfered
                                    ));
                                    transfer_state.send_state.cur_bytes_transfered = new_pos;
                                    if let Some(cur) = &mut self.cur_buf {
                                        cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                                    }
                                    return Ok(()); // Continue sending from new position
                                }
                                _ => {
                                    transfer_state
                                        .send_state
                                        .log_error(format!("Unexpected header after final data: {:?}", header.frame_type));
                                    Zmodem::cancel(com).await?;
                                    transfer_state.is_finished = true;
                                    return Ok(());
                                }
                            }
                        }
                    }
                    // Now send ZEOF
                    Header::from_number(ZFrameType::Eof, transfer_state.send_state.file_size as u32)
                        .write(com, self.get_header_type(), self.can_esc_control())
                        .await?;
                    transfer_state.send_state.log_info("ZEOF sent, waiting for ZRINIT".to_string());
                    self.state = SendState::Await;
                    return Ok(());
                }

                if !self.nonstop {
                    // Handle ACK for mid-file packets
                    let ack = Header::read(com, &mut self.can_count).await?;
                    if let Some(header) = ack {
                        match header.frame_type {
                            ZFrameType::Ack => {
                                // ok, continue - could log if verbose
                            }
                            ZFrameType::Nak => {
                                transfer_state
                                    .send_state
                                    .log_warning(format!("NAK received, resending from position {}", old_pos));
                                transfer_state.send_state.cur_bytes_transfered = old_pos; /* resend */
                                if let Some(cur) = &mut self.cur_buf {
                                    cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                                }
                            }
                            ZFrameType::RPos => {
                                let new_pos = header.number() as u64;
                                transfer_state.send_state.log_warning(format!(
                                    "RPOS received, repositioning from {} to {}",
                                    transfer_state.send_state.cur_bytes_transfered, new_pos
                                ));
                                transfer_state.send_state.cur_bytes_transfered = new_pos;
                                if let Some(cur) = &mut self.cur_buf {
                                    cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                                }
                                self.state = SendState::SendZDATA; // Restart data transmission
                            }
                            _ => {
                                transfer_state
                                    .send_state
                                    .log_error(format!("Unexpected header during data transfer: {:?}", header.frame_type));
                                Zmodem::cancel(com).await?;
                                transfer_state.is_finished = true;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn read_next_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        let err = Header::read(com, &mut self.can_count).await;
        if self.can_count >= 5 {
            transfer_state.send_state.log_error("Received 5+ CAN bytes, cancelling".to_string());
            Zmodem::cancel(com).await?;
            transfer_state.is_finished = true;
            return Ok(());
        }
        match err {
            Err(err) => {
                self.errors += 1;
                transfer_state
                    .send_state
                    .log_error(format!("Error reading header (error #{}/3): {:?}", self.errors, err));
                if self.errors > 3 {
                    transfer_state.send_state.log_error("Too many header errors, aborting".to_string());
                    Zmodem::cancel(com).await?;
                    transfer_state.is_finished = true;
                    return Err(err);
                }
                return Ok(());
            }
            Ok(Some(res)) => {
                return self.handle_header(com, transfer_state, res).await;
            }

            Ok(None) => {
                transfer_state.send_state.log_warning("No header received".to_string());
            }
        }
        Ok(())
    }

    async fn handle_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState, res: Header) -> crate::Result<()> {
        self.errors = 0;
        Ok(match res.frame_type {
            ZFrameType::RIinit => {
                if self.cur_buf.is_some() {
                    // File transfer completed successfully
                    transfer_state
                        .send_state
                        .log_info(format!("File transfer confirmed: {}", self.cur_file.display()));
                    transfer_state.send_state.finish_file(self.cur_file.clone());
                    self.transfered_file = true;
                    self.cur_buf = None;
                    self.state = SendState::SendNextFile;
                    return Ok(());
                }

                // Log receiver capabilities
                let mut capabilities = Vec::new();
                if self.can_fdx() {
                    capabilities.push("FDX");
                }
                if self.can_receive_data_during_io() {
                    capabilities.push("OVIO");
                }
                if self.can_use_crc32() {
                    capabilities.push("CRC32");
                }
                if self.can_esc_control() {
                    capabilities.push("ESCCTL");
                }

                transfer_state.send_state.cur_bytes_transfered = 0;
                self.receiver_capabilities = res.f0();
                let block_size = res.p0() as usize + ((res.p1() as usize) << 8);
                self.nonstop = block_size == 0;

                transfer_state.send_state.log_info(format!(
                    "ZRINIT received - Capabilities: [{}], Block size: {}, Mode: {}",
                    capabilities.join(", "),
                    if block_size == 0 { "unlimited".to_string() } else { block_size.to_string() },
                    if self.nonstop { "streaming" } else { "segmented" }
                ));

                if block_size != 0 {
                    self.package_len = block_size;
                }
                self.state = SendState::SendNextFile;
                return Ok(());
            }

            ZFrameType::Nak => {
                transfer_state.send_state.log_warning("NAK received, will resend file header".to_string());
            }

            ZFrameType::RPos => {
                let new_pos = res.number() as u64;
                transfer_state
                    .send_state
                    .log_info(format!("RPOS received, repositioning to offset {}", new_pos));
                transfer_state.send_state.cur_bytes_transfered = new_pos;
                if let Some(cur) = &mut self.cur_buf {
                    cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                }

                self.state = SendState::SendZDATA;

                if let SendState::SendDataPackages = self.state {
                    if self.package_len > 512 {
                        transfer_state
                            .send_state
                            .log_info(format!("Reducing packet size from {} to {}", self.package_len, self.package_len / 2));
                        //reinit transfer.
                        self.package_len /= 2;
                        self.state = SendState::SendZRQInit;
                        return Ok(());
                    }
                }
            }

            ZFrameType::Fin => {
                transfer_state.send_state.log_info("ZFIN received, ending session".to_string());
                transfer_state.is_finished = true;
                com.send(b"OO").await?;
                return Ok(());
            }
            ZFrameType::Challenge => {
                transfer_state.send_state.log_info(format!("Challenge received: 0x{:08x}", res.number()));
                Header::from_number(ZFrameType::Ack, res.number())
                    .write(com, self.get_header_type(), self.can_esc_control())
                    .await?;
            }
            ZFrameType::Abort | ZFrameType::FErr | ZFrameType::Can => {
                transfer_state.send_state.log_error(format!("Session abort requested: {:?}", res.frame_type));
                Header::empty(ZFrameType::Fin)
                    .write(com, self.get_header_type(), self.can_esc_control())
                    .await?;
                transfer_state.is_finished = true;
            }
            unk_frame => {
                transfer_state.send_state.log_error(format!("Unsupported frame type: {:?}", unk_frame));
                return Err(ZModemError::UnsupportedFrame(unk_frame).into());
            }
        })
    }

    async fn send_zfile(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState, mut tries: i32) -> crate::Result<()> {
        loop {
            // Replace recursion with a loop
            if self.cur_buf.is_none() {
                transfer_state.send_state.log_error("No file buffer for ZFILE".to_string());
                transfer_state.is_finished = true;
                return Ok(());
            }

            transfer_state.send_state.log_info(format!(
                "Sending ZFILE header for '{}' (attempt {}/5)",
                transfer_state.send_state.file_name,
                tries + 1
            ));

            let mut b =
                Header::from_flags(ZFrameType::File, 0, 0, zfile_flag::ZMNEW, zfile_flag::ZCRESUM).build(self.get_header_type(), self.can_esc_control());

            let data = { format!("{}\0{}\0", transfer_state.send_state.file_name, transfer_state.send_state.file_size).into_bytes() };

            b.extend_from_slice(&self.encode_subpacket(ZCRCW, &data));
            com.send(&b).await?;
            let ack = Header::read(com, &mut self.can_count).await?;
            if let Some(header) = ack {
                match header.frame_type {
                    ZFrameType::Ack => {
                        transfer_state.send_state.log_info("ZFILE accepted, ready to send data".to_string());
                        self.state = SendState::SendZDATA;
                        break; // Exit loop on success
                    }
                    ZFrameType::Skip => {
                        transfer_state
                            .send_state
                            .log_info(format!("File '{}' skipped by receiver", transfer_state.send_state.file_name));
                        self.state = SendState::SendNextFile;
                        break; // Exit loop
                    }
                    ZFrameType::RPos => {
                        let resume_pos = header.number() as u64;
                        transfer_state.send_state.log_info(format!("Resuming transfer at position {}", resume_pos));
                        transfer_state.send_state.cur_bytes_transfered = resume_pos;
                        if let Some(cur) = &mut self.cur_buf {
                            cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                        }
                        self.state = SendState::SendZDATA;
                        break; // Exit loop
                    }
                    ZFrameType::Nak => {
                        tries += 1;
                        if tries > 5 {
                            transfer_state.send_state.log_error("Too many NAKs for ZFILE, aborting".to_string());
                            Zmodem::cancel(com).await?;
                            transfer_state.is_finished = true;
                            return Ok(());
                        }
                        transfer_state.send_state.log_warning(format!("NAK received for ZFILE, retry {}/5", tries));
                        // Continue loop to retry
                        continue;
                    }
                    ZFrameType::Fin => {
                        transfer_state.send_state.log_info("Receiver sent ZFIN, ending session".to_string());
                        com.send(b"OO").await?;
                        transfer_state.is_finished = true;
                        break; // Exit loop
                    }
                    _ => {
                        transfer_state
                            .send_state
                            .log_error(format!("Unexpected response to ZFILE: {:?}", header.frame_type));
                        // cancel
                        Zmodem::cancel(com).await?;
                        transfer_state.is_finished = true;
                        break; // Exit loop
                    }
                }
            } else {
                tries += 1;
                if tries > 5 {
                    transfer_state
                        .send_state
                        .log_error("No response to ZFILE after 5 attempts, aborting".to_string());
                    Zmodem::cancel(com).await?;
                    transfer_state.is_finished = true;
                    break;
                }
                transfer_state.send_state.log_warning(format!("No response to ZFILE, retry {}/5", tries));
                continue;
            }
        }

        transfer_state.send_state.cur_bytes_transfered = 0;
        Ok(())
    }

    pub fn send(&mut self, files: &[PathBuf]) {
        self.state = SendState::SendZRQInit;
        for f in files {
            self.file_queue.push_back(f.clone());
        }
        self.retries = 0;
    }

    pub async fn send_zrqinit(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.cur_buf = None;
        self.transfered_file = true;
        Header::empty(ZFrameType::RQInit)
            .write(com, self.get_header_type(), self.can_esc_control())
            .await?;
        Ok(())
    }

    pub async fn send_zfin(&mut self, com: &mut dyn Connection, size: u32) -> crate::Result<()> {
        Header::from_number(ZFrameType::Fin, size)
            .write(com, self.get_header_type(), self.can_esc_control())
            .await?;
        self.state = SendState::Await;
        Ok(())
    }
}
