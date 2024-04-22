#![allow(clippy::unused_self)]

use std::{
    cmp::min,
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read, Seek},
    path::PathBuf,
};

use crate::{
    protocol::{zfile_flag, zmodem::err::ZModemError, Header, HeaderType, TransferState, ZFrameType, Zmodem, ZCRCE, ZCRCG},
    Connection,
};

use super::{ZCRCQ, ZCRCW};

#[derive(Debug)]
pub enum SendState {
    Await,
    SendZRQInit,
    SendZDATA,
    SendDataPackages,
    Finished,
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
            state: SendState::Finished,
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
        if self.can_use_crc32() {
            HeaderType::Bin32
        } else {
            HeaderType::Bin
        }
    }

    fn encode_subpacket(&self, zcrc_byte: u8, data: &[u8]) -> Vec<u8> {
        if self.can_use_crc32() {
            Zmodem::encode_subpacket_crc32(zcrc_byte, data, self.can_esc_control())
        } else {
            Zmodem::encode_subpacket_crc16(zcrc_byte, data, self.can_esc_control())
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.state, SendState::Finished)
    }

    fn next_file(&mut self, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let Some(next_file) = self.file_queue.pop_front() {
            transfer_state.send_state.file_name = next_file.file_name().unwrap().to_string_lossy().to_string();
            transfer_state.send_state.file_size = next_file.metadata()?.len();

            self.cur_file = next_file.clone();
            let reader = BufReader::new(File::open(next_file)?);
            self.cur_buf = Some(reader);
        } else {
            transfer_state.is_finished = true;
        }
        Ok(())
    }

    pub fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let SendState::Finished = self.state {
            return Ok(());
        }
        if self.retries > 5 {
            Zmodem::cancel(com)?;
            self.state = SendState::Finished;
            return Ok(());
        }
        transfer_state.update_time();
        let transfer_info = &mut transfer_state.send_state;
        transfer_info.errors = self.errors;
        transfer_info.check_size = format!("Crc32/{}", self.package_len);
        transfer_info.update_bps();
        match self.state {
            SendState::Await => {
                self.read_next_header(com, transfer_state)?;
            }
            SendState::SendZRQInit => {
                //                transfer_state.current_state = "Negotiating transfer";
                //    let now = Instant::now();
                //     if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                self.send_zrqinit(com)?;
                self.state = SendState::Await;
                self.retries += 1;
                //         self.last_send = Instant::now();
                //     }
            }
            SendState::SendZDATA => {
                //              transfer_state.current_state = "Sending data";
                if self.cur_buf.is_none() {
                    transfer_state.is_finished = true;
                    //println!("no file to send!");
                    return Ok(());
                }
                Header::from_number(ZFrameType::Data, transfer_state.send_state.cur_bytes_transfered as u32).write(
                    com,
                    self.get_header_type(),
                    self.can_esc_control(),
                )?;
                self.state = SendState::SendDataPackages;
            }
            SendState::SendDataPackages => {
                let mut p = Vec::new();
                if self.cur_buf.is_none() {
                    return Ok(());
                }
                let old_pos = transfer_state.send_state.cur_bytes_transfered;
                let end_pos = min(
                    transfer_state.send_state.file_size as usize,
                    transfer_state.send_state.cur_bytes_transfered as usize + self.package_len,
                );
                let crc_byte = if transfer_state.send_state.cur_bytes_transfered as usize + self.package_len < transfer_state.send_state.file_size as usize {
                    if self.nonstop {
                        ZCRCG
                    } else {
                        ZCRCQ
                    }
                } else if self.nonstop {
                    ZCRCE
                } else {
                    ZCRCW
                };

                if let Some(cur) = &mut self.cur_buf {
                    let mut block = vec![0; self.package_len];
                    let bytes = cur.read(&mut block)?;
                    p.extend_from_slice(&self.encode_subpacket(crc_byte, &block));
                    transfer_state.send_state.total_bytes_transfered += bytes as u64;
                    transfer_state.send_state.cur_bytes_transfered += bytes as u64;
                } else {
                    return Err(ZModemError::NoFileOpen.into());
                }

                if transfer_state.send_state.cur_bytes_transfered >= transfer_state.send_state.file_size {
                    p.extend_from_slice(&Header::from_number(ZFrameType::Eof, end_pos as u32).build(self.get_header_type(), self.can_esc_control()));
                    // println!("send eof!");
                    //transfer_info.write("Done sending file date.".to_string());
                    // transfer_state.current_state = "Done data";
                    self.transfered_file = true;
                    self.state = SendState::Await;
                }
                com.write_all(&p)?;
                if !self.nonstop {
                    let ack = Header::read(com, &mut self.can_count)?;
                    if let Some(header) = ack {
                        // println!("got header after data package: {header}",);
                        match header.frame_type {
                            ZFrameType::Ack => { /* ok */ }
                            ZFrameType::Nak => {
                                transfer_state.send_state.cur_bytes_transfered = old_pos; /* resend */
                                if let Some(cur) = &mut self.cur_buf {
                                    cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                                }
                            }
                            ZFrameType::RPos => {
                                transfer_state.send_state.cur_bytes_transfered = header.number() as u64;
                                if let Some(cur) = &mut self.cur_buf {
                                    cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                                }
                            }
                            _ => {
                                log::error!("unexpected header {header:?}");
                                // cancel
                                self.state = SendState::Finished;
                                Zmodem::cancel(com)?;
                            }
                        }
                    }
                }
            }
            SendState::Finished => {
                //                transfer_state.current_state = "Finishing transfer…";
                // let now = Instant::now();
                //if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                self.send_zfin(com, 0)?;
                //}
                return Ok(());
            }
        }
        Ok(())
    }

    fn read_next_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        let err = Header::read(com, &mut self.can_count);
        if self.can_count >= 5 {
            // transfer_info.write("Received cancel...".to_string());
            self.state = SendState::Finished;
            return Ok(());
        }
        if let Err(err) = err {
            log::error!("error reading header: {:?}", err);
            if self.errors > 3 {
                self.state = SendState::Finished;
                Zmodem::cancel(com)?;
                return Err(err);
            }
            self.errors += 1;
            return Ok(());
        }
        self.errors = 0;
        let res = err.unwrap();
        if let Some(res) = res {
            // println!("got header {}", res);
            match res.frame_type {
                ZFrameType::RIinit => {
                    if self.transfered_file {
                        self.next_file(transfer_state)?;
                        self.transfered_file = false;
                    }

                    if transfer_state.is_finished {
                        self.send_zfin(com, transfer_state.send_state.cur_bytes_transfered as u32)?;
                        transfer_state.send_state.cur_bytes_transfered = 0;
                        return Ok(());
                    }
                    transfer_state.send_state.cur_bytes_transfered = 0;
                    self.receiver_capabilities = res.f0();
                    let block_size = res.p0() as usize + ((res.p1() as usize) << 8);
                    self.nonstop = block_size == 0;
                    if block_size != 0 {
                        self.package_len = block_size;
                    }
                    /*
                    if self._can_decrypt() {
                        println!("receiver can decrypt");
                    }
                    if self.can_fdx() {
                        println!("receiver can send and receive true full duplex");
                    }
                    if self._can_receive_data_during_io() {
                        println!("receiver can receive data during disk I/O");
                    }
                    if self._can_send_break() {
                        println!("receiver can send a break signal");
                    }
                    if self._can_lzw() {
                        println!("receiver can uncompress");
                    }
                    if self.can_use_crc32() {
                        println!("receiver can use 32 bit Frame Check");
                    }
                    if self.can_esc_control() {
                        println!("receiver expects ctl chars to be escaped");
                    }
                    if self._can_esc_8thbit() {
                        println!("receiver expects 8th bit to be escaped");
                    }*/
                    //  transfer_state.current_state = "Sending header";
                    self.send_zfile(com, transfer_state, 0)?;
                    return Ok(());
                }

                ZFrameType::Nak => {
                    // transfer_info
                    //     .write("Package error, resending file header...".to_string());
                }

                ZFrameType::RPos => {
                    transfer_state.send_state.cur_bytes_transfered = res.number() as u64;
                    if let Some(cur) = &mut self.cur_buf {
                        cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                    }

                    self.state = SendState::SendZDATA;

                    if let SendState::SendDataPackages = self.state {
                        if self.package_len > 512 {
                            //reinit transfer.
                            self.package_len /= 2;
                            self.state = SendState::SendZRQInit;
                            return Ok(());
                        }
                    }
                }

                ZFrameType::Fin => {
                    self.state = SendState::Finished;
                    com.write_all(b"OO")?;
                    return Ok(());
                }
                ZFrameType::Challenge => {
                    Header::from_number(ZFrameType::Ack, res.number()).write(com, self.get_header_type(), self.can_esc_control())?;
                }
                ZFrameType::Abort | ZFrameType::FErr | ZFrameType::Can => {
                    Header::empty(ZFrameType::Fin).write(com, self.get_header_type(), self.can_esc_control())?;
                    self.state = SendState::Finished;
                }
                unk_frame => {
                    return Err(ZModemError::UnsupportedFrame(unk_frame).into());
                }
            }
        }
        Ok(())
    }

    fn send_zfile(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState, tries: i32) -> crate::Result<()> {
        if self.cur_buf.is_none() {
            self.state = SendState::Finished;
            return Ok(());
        }
        let mut b = Vec::new();
        //transfer_state.write("Send file header".to_string());
        // println!("send zfile!");
        b.extend_from_slice(
            &Header::from_flags(ZFrameType::File, 0, 0, zfile_flag::ZMNEW, zfile_flag::ZCRESUM).build(self.get_header_type(), self.can_esc_control()),
        );
        let data =/*  if f.date > 0 {
            format!(
                "{}\0{} {} 0 0 {} {}\0",
                transfer_state.send_state.file_name,
                transfer_state.send_state.file_size,
                f.date,
                self.files.len() - cur_file_size,
                bytes_left
            )
            .into_bytes()
        } else*/ {
            format!("{}\0{}\0", transfer_state.send_state.file_name, transfer_state.send_state.file_size).into_bytes()
        };

        b.extend_from_slice(&self.encode_subpacket(ZCRCW, &data));
        com.write_all(&b)?;

        let ack = Header::read(com, &mut self.can_count)?;
        if let Some(header) = ack {
            // println!("got header afer zfile: {}", header);
            match header.frame_type {
                ZFrameType::Ack => self.state = SendState::SendZDATA,
                ZFrameType::Skip => {
                    self.next_file(transfer_state)?;
                    self.send_zfile(com, transfer_state, 0)?;
                }
                ZFrameType::RPos => {
                    transfer_state.send_state.cur_bytes_transfered = header.number() as u64;
                    if let Some(cur) = &mut self.cur_buf {
                        cur.seek(std::io::SeekFrom::Start(transfer_state.send_state.cur_bytes_transfered))?;
                    }
                    self.state = SendState::SendZDATA;
                }

                ZFrameType::Nak => {
                    if tries > 5 {
                        log::error!("too many tries for z_file");
                        self.state = SendState::Finished;
                        Zmodem::cancel(com)?;
                        return Ok(());
                    }
                    // self.send_zfile(com, tries + 1); /* resend */
                }
                _ => {
                    log::error!("unexpected header {header:?}");
                    // cancel
                    self.state = SendState::Finished;
                    Zmodem::cancel(com)?;
                }
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

    pub fn send_zrqinit(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.cur_buf = None;
        self.transfered_file = true;
        Header::empty(ZFrameType::RQInit).write(com, self.get_header_type(), self.can_esc_control())?;
        Ok(())
    }

    pub fn send_zfin(&mut self, com: &mut dyn Connection, size: u32) -> crate::Result<()> {
        Header::from_number(ZFrameType::Fin, size).write(com, self.get_header_type(), self.can_esc_control())?;
        self.state = SendState::Await;
        Ok(())
    }
}
