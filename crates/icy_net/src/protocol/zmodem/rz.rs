#![allow(clippy::unused_self, clippy::wildcard_imports)]
use std::{cmp::Ordering, io::Write, time::Instant};
use tempfile::NamedTempFile;

use crate::{
    Connection,
    crc::{get_crc16_buggy_zlde, get_crc32, update_crc32},
    protocol::{Header, HeaderType, TransferState, ZCRCE, ZCRCG, ZCRCW, ZFrameType, Zmodem, str_from_null_terminated_utf8_unchecked},
};

use super::{constants::*, err::ZModemError, read_zdle_bytes, zrinit_flag::CANFDX};

#[derive(Debug)]
pub enum RecvState {
    Idle,
    Await,
    AwaitZDATA,
    AwaitFileData,
    AwaitEOF,
    SendZRINIT,
}

pub struct Rz {
    state: RecvState,
    retries: usize,
    can_count: usize,
    block_length: usize,
    sender_flags: u8,
    use_crc32: bool,
    last_send: Instant,

    cur_out_file: Option<NamedTempFile>,

    can_fullduplex: bool,
    can_esc_control: bool,
    no_streaming: bool,
    can_break: bool,
    want_fcs_16: bool,
    escape_8th_bit: bool,
    attn_seq: Vec<u8>,
}

impl Rz {
    pub fn new(block_length: usize) -> Self {
        Self {
            state: RecvState::Idle,
            block_length,
            retries: 0,
            can_count: 0,
            sender_flags: 0,
            use_crc32: true,
            last_send: Instant::now(),
            can_fullduplex: true,
            can_esc_control: false,
            can_break: false,
            no_streaming: false,
            want_fcs_16: true,
            escape_8th_bit: false,
            attn_seq: vec![0],
            cur_out_file: None,
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.state, RecvState::Idle)
    }

    async fn cancel(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.state = RecvState::Idle;
        Zmodem::cancel(com).await
    }

    pub async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let RecvState::Idle = self.state {
            return Ok(());
        }
        if self.retries > 5 {
            transfer_state
                .recieve_state
                .log_error(format!("Too many retries ({}), cancelling transfer", self.retries));
            self.cancel(com).await?;
            return Ok(());
        }
        transfer_state.update_time();
        let transfer_info = &mut transfer_state.recieve_state;
        transfer_info.check_size = if self.use_crc32 { "CRC32" } else { "CRC16" }.to_string();
        transfer_info.update_bps();

        println!(
            "update transfer: RZ State: {:?}, Received: {} / {} bytes ({} bps)",
            self.state, transfer_info.cur_bytes_transfered, transfer_info.file_size, transfer_info.bps
        );

        match self.state {
            RecvState::SendZRINIT => {
                if self.read_header(com, transfer_state).await? {
                    return Ok(());
                }
                /*  let now = Instant::now();
                 if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zrinit(com)?;
                    self.retries += 1;
                    self.last_send = Instant::now();
                }*/
            }
            /*     RecvState::AwaitZDATA => {
                self.read_header(com, storage_handler, transfer_state)?;
            }*/
            RecvState::AwaitFileData => {
                let pck = read_subpacket(com, self.block_length, self.use_crc32, self.can_esc_control).await;
                match pck {
                    Ok((block, is_last, expect_ack)) => {
                        // Write data first, then send ACK
                        if let Some(named_file) = &mut self.cur_out_file {
                            named_file.as_file_mut().write_all(&block)?;
                            transfer_state.recieve_state.total_bytes_transfered += block.len() as u64;
                            transfer_state.recieve_state.cur_bytes_transfered += block.len() as u64;
                        } else {
                            transfer_state.recieve_state.log_error("No file open for writing data".to_string());
                            return Err(ZModemError::NoFileOpen.into());
                        }

                        if expect_ack {
                            // Send ACK with correct offset (not block length)
                            Header::from_number(ZFrameType::Ack, transfer_state.recieve_state.cur_bytes_transfered as u32)
                                .write(com, HeaderType::Hex, self.can_esc_control)
                                .await?;

                            transfer_state
                                .recieve_state
                                .log_info(format!("Sent ACK for offset {}", transfer_state.recieve_state.cur_bytes_transfered));
                        }

                        if is_last {
                            transfer_state.recieve_state.log_info("Last data subpacket received, awaiting ZEOF".to_string());
                            self.state = RecvState::AwaitEOF;
                        }
                    }
                    Err(err) => {
                        transfer_state.recieve_state.errors += 1;
                        log::error!("{err}");
                        transfer_state.recieve_state.log_error(format!(
                            "Subpacket error #{}: {}, requesting retransmission from offset {}",
                            transfer_state.recieve_state.errors, err, transfer_state.recieve_state.cur_bytes_transfered
                        ));

                        Header::from_number(ZFrameType::RPos, transfer_state.recieve_state.cur_bytes_transfered as u32)
                            .write(com, HeaderType::Hex, self.can_esc_control)
                            .await?;
                        self.state = RecvState::AwaitZDATA;
                        return Ok(());
                    }
                }
            }
            _ => {
                self.read_header(com, transfer_state).await?;
            }
        }
        Ok(())
    }

    async fn request_zpos(&mut self, com: &mut dyn Connection, pos: u32) -> crate::Result<usize> {
        Header::from_number(ZFrameType::RPos, pos)
            .write(com, HeaderType::Hex, self.can_esc_control)
            .await
    }

    async fn read_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<bool> {
        match Header::read(com, &mut self.can_count).await {
            Ok(Some(header)) => {
                self.can_count = 0;
                return self.handle_header(com, transfer_state, header).await;
            }

            Ok(None) => {
                return Ok(false);
            }

            Err(err) => {
                if self.can_count >= 5 {
                    transfer_state.recieve_state.log_error("Received 5+ CAN bytes, cancelling session".to_string());
                    self.cancel(com).await?;
                    self.cancel(com).await?;
                    self.cancel(com).await?;
                    self.state = RecvState::Idle;
                    return Ok(false);
                }
                transfer_state.recieve_state.errors += 1;
                transfer_state
                    .recieve_state
                    .log_warning(format!("Header read error #{}: {:?}", transfer_state.recieve_state.errors, err));
                return Ok(false);
            }
        }
    }

    async fn handle_header(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState, header: Header) -> crate::Result<bool> {
        self.use_crc32 = matches!(header.header_type, HeaderType::Bin32 | HeaderType::Hex);
        match header.frame_type {
            ZFrameType::Sinit => {
                transfer_state.recieve_state.log_info("ZSINIT received, reading attention sequence".to_string());
                let pck = read_subpacket(com, self.block_length, self.use_crc32, self.can_esc_control).await;
                match pck {
                    Ok(attn_seq) => {
                        self.attn_seq = attn_seq.0;
                        self.sender_flags = header.f0();

                        // Log sender flags
                        let mut flags = Vec::new();
                        if self.sender_flags & super::zsinit_flag::TESCCTL != 0 {
                            flags.push("ESCCTL");
                        }
                        if self.sender_flags & super::zsinit_flag::TESC8 != 0 {
                            flags.push("ESC8");
                        }

                        transfer_state.recieve_state.log_info(format!(
                            "ZSINIT accepted - Sender flags: [{}], Attn seq length: {} bytes",
                            flags.join(", "),
                            self.attn_seq.len()
                        ));

                        Header::empty(ZFrameType::Ack).write(com, HeaderType::Hex, self.can_esc_control).await?;
                        return Ok(true);
                    }
                    Err(err) => {
                        log::error!("{err}");
                        transfer_state.recieve_state.log_error(format!("Failed to read ZSINIT data: {}", err));
                        Header::empty(ZFrameType::Nak).write(com, HeaderType::Hex, self.can_esc_control).await?;
                        return Ok(false);
                    }
                }
            }

            ZFrameType::RQInit => {
                transfer_state.recieve_state.log_info("ZRQINIT received, will send ZRINIT".to_string());
                self.state = RecvState::SendZRINIT;
                return Ok(true);
            }
            ZFrameType::File => {
                transfer_state.recieve_state.log_info("ZFILE header received, reading file info".to_string());
                let pck = read_subpacket(com, self.block_length, self.use_crc32, self.can_esc_control).await;

                match pck {
                    Ok((block, _, _)) => {
                        let file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
                        let mut file_size = 0;
                        for b in &block[(file_name.len() + 1)..] {
                            if *b < b'0' || *b > b'9' {
                                break;
                            }
                            file_size = file_size * 10 + (*b - b'0') as usize;
                        }

                        transfer_state
                            .recieve_state
                            .log_info(format!("Starting file transfer: '{}' ({} bytes)", file_name, file_size));

                        transfer_state.recieve_state.file_name = file_name;
                        self.cur_out_file = Some(NamedTempFile::new()?);
                        transfer_state.recieve_state.file_size = file_size as u64;
                        transfer_state.recieve_state.cur_bytes_transfered = 0;

                        self.state = RecvState::AwaitZDATA;

                        transfer_state
                            .recieve_state
                            .log_info(format!("Requesting start position: {}", transfer_state.recieve_state.cur_bytes_transfered));

                        self.request_zpos(com, transfer_state.recieve_state.cur_bytes_transfered as u32).await?;
                        return Ok(true);
                    }
                    Err(err) => {
                        log::error!("{err}");
                        transfer_state.recieve_state.errors += 1;
                        transfer_state
                            .recieve_state
                            .log_error(format!("Failed to read file info: {}, sending attention sequence", err));
                        com.send(&self.attn_seq).await?;
                        Header::from_number(ZFrameType::RPos, transfer_state.recieve_state.cur_bytes_transfered as u32)
                            .write(com, HeaderType::Hex, self.can_esc_control)
                            .await?;
                        return Ok(false);
                    }
                }
            }
            ZFrameType::Data => {
                let offset = header.number();
                transfer_state.recieve_state.log_info(format!("ZDATA received at offset {}", offset));

                if self.cur_out_file.is_none() {
                    transfer_state.recieve_state.log_error("ZDATA received before ZFILE".to_string());
                    self.cancel(com).await?;
                    return Err(ZModemError::ZDataBeforeZFILE.into());
                }
                let len = transfer_state.recieve_state.cur_bytes_transfered as usize;
                match len.cmp(&(offset as usize)) {
                    Ordering::Greater => {
                        transfer_state
                            .recieve_state
                            .log_warning(format!("Sender offset {} is behind our position {}, truncating file", offset, len));
                        if let Some(named_file) = self.cur_out_file.take() {
                            named_file.as_file().set_len(offset as u64)?;
                            transfer_state.recieve_state.cur_bytes_transfered = offset as u64;
                            self.cur_out_file = Some(named_file);
                        } else {
                            return Err(ZModemError::NoFileOpen.into());
                        }
                    }

                    Ordering::Less => {
                        transfer_state
                            .recieve_state
                            .log_warning(format!("Sender offset {} ahead of our position {}, requesting resync", offset, len));
                        Header::from_number(ZFrameType::RPos, len as u32)
                            .write(com, HeaderType::Hex, self.can_esc_control)
                            .await?;
                        return Ok(false);
                    }
                    Ordering::Equal => {
                        transfer_state.recieve_state.log_info("Data offset matches, ready to receive".to_string());
                    }
                }
                self.state = RecvState::AwaitFileData;
                return Ok(true);
            }
            ZFrameType::Eof => {
                let expected_size = header.number() as u64;
                transfer_state.recieve_state.log_info(format!(
                    "ZEOF received, file size: {} (received: {})",
                    expected_size, transfer_state.recieve_state.cur_bytes_transfered
                ));

                if transfer_state.recieve_state.cur_bytes_transfered != expected_size {
                    transfer_state.recieve_state.log_warning(format!(
                        "File size mismatch! Expected {}, got {}. Requesting missing data.",
                        expected_size, transfer_state.recieve_state.cur_bytes_transfered
                    ));
                    Header::from_number(ZFrameType::RPos, transfer_state.recieve_state.cur_bytes_transfered as u32)
                        .write(com, HeaderType::Hex, self.can_esc_control)
                        .await?;
                    return Ok(false);
                }

                self.send_zrinit(com).await?;
                transfer_state.recieve_state.log_info(format!(
                    "File '{}' successfully received ({} bytes)",
                    transfer_state.recieve_state.file_name, transfer_state.recieve_state.cur_bytes_transfered
                ));

                if let Some(named_file) = self.cur_out_file.take() {
                    let path = &named_file.keep()?.1;
                    transfer_state.recieve_state.finish_file(path.clone());
                } else {
                    return Err(ZModemError::NoFileOpen.into());
                }
                self.state = RecvState::SendZRINIT;
                return Ok(true);
            }
            ZFrameType::Fin => {
                transfer_state.recieve_state.log_info("ZFIN received, session ending".to_string());
                Header::empty(ZFrameType::Fin).write(com, HeaderType::Hex, self.can_esc_control).await?;
                self.state = RecvState::Idle;
                return Ok(true);
            }
            ZFrameType::Challenge => {
                transfer_state
                    .recieve_state
                    .log_info(format!("Challenge received: 0x{:08x}, sending response", header.number()));
                Header::from_number(ZFrameType::Ack, header.number())
                    .write(com, HeaderType::Hex, self.can_esc_control)
                    .await?;
                return Ok(true);
            }
            ZFrameType::FreeCnt => {
                transfer_state
                    .recieve_state
                    .log_info("Free space request received, reporting unlimited".to_string());
                // 0 means unlimited space but sending free hd space to an unknown source is a security issue
                Header::from_number(ZFrameType::Ack, 0)
                    .write(com, HeaderType::Hex, self.can_esc_control)
                    .await?;
                return Ok(true);
            }
            ZFrameType::Command => {
                // just protocol it.
                let package = read_subpacket(com, self.block_length, self.use_crc32, self.can_esc_control).await;
                match &package {
                    Ok((block, _, _)) => {
                        let cmd = str_from_null_terminated_utf8_unchecked(block);
                        transfer_state
                            .recieve_state
                            .log_error(format!("SECURITY: Remote attempted to execute command: '{}' (REJECTED)", cmd));
                        log::error!("Remote wanted to execute {cmd} on the system. (did not execute)");
                    }
                    Err(err) => {
                        transfer_state.recieve_state.log_error(format!("Failed to read command packet: {}", err));
                        log::error!("{err}");
                    }
                }
                Header::from_number(ZFrameType::Compl, 0)
                    .write(com, HeaderType::Hex, self.can_esc_control)
                    .await?;
                return Ok(true);
            }
            ZFrameType::Abort | ZFrameType::FErr | ZFrameType::Can => {
                transfer_state
                    .recieve_state
                    .log_error(format!("Abort signal received: {:?}", header.frame_type));
                Header::empty(ZFrameType::Fin).write(com, HeaderType::Hex, self.can_esc_control).await?;
                self.state = RecvState::Idle;
                return Ok(false);
            }
            unk_frame => {
                transfer_state.recieve_state.log_error(format!("Unsupported frame type: {:?}", unk_frame));
                return Err(ZModemError::UnsupportedFrame(unk_frame).into());
            }
        }
    }

    pub async fn recv(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        self.state = RecvState::Await;
        self.retries = 0;
        self.send_zrinit(com).await?;
        Ok(())
    }

    pub async fn send_zrinit(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        let mut flags = 0;
        let mut capabilities = Vec::new();

        if self.can_fullduplex {
            flags |= CANFDX;
            capabilities.push("FDX");
        }
        if !self.no_streaming {
            flags |= zrinit_flag::CANOVIO;
            capabilities.push("OVIO");
        }
        if self.can_break {
            flags |= zrinit_flag::CANBRK;
            capabilities.push("BRK");
        }
        if self.want_fcs_16 {
            flags |= zrinit_flag::CANFC32;
            capabilities.push("CRC32");
        }
        if self.can_esc_control {
            flags |= zrinit_flag::ESCCTL;
            capabilities.push("ESCCTL");
        }
        if self.escape_8th_bit {
            flags |= zrinit_flag::ESC8;
            capabilities.push("ESC8");
        }

        // Note: we don't have access to transfer_state here, so we can't log
        // Consider passing it as parameter if logging is needed

        Header::from_flags(ZFrameType::RIinit, 0, 0, 0, flags)
            .write(com, HeaderType::Hex, self.can_esc_control)
            .await?;
        Ok(())
    }
}

pub async fn read_subpacket(com: &mut dyn Connection, block_length: usize, use_crc32: bool, escape_ctrl_chars: bool) -> crate::Result<(Vec<u8>, bool, bool)> {
    let mut data = Vec::with_capacity(block_length);
    loop {
        match read_zdle_byte(com, escape_ctrl_chars).await? {
            ZModemResult::Ok(b) => data.push(b),
            ZModemResult::CrcCheckRequested(first_byte, frame_ends, zack_requested) => match check_crc(com, use_crc32, &data, first_byte).await {
                Ok(_) => {
                    return Ok((data, frame_ends, zack_requested));
                }
                Err(err) => {
                    return Err(ZModemError::SubpacketCrcError(err.to_string()).into());
                }
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ZModemResult {
    Ok(u8),

    /// first bool:frame ends
    /// second bool:zack requested
    CrcCheckRequested(u8, bool, bool),
}

pub async fn read_zdle_byte(com: &mut dyn Connection, escape_ctrl_chars: bool) -> crate::Result<ZModemResult> {
    loop {
        let c = com.read_u8().await?;
        match c {
            ZDLE => {
                loop {
                    let c = com.read_u8().await?;
                    match c {
                        XON | XON_0X80 | XOFF | XOFF_0X80 | ZDLE => {
                            continue;
                        }
                        ZRUB0 => return Ok(ZModemResult::Ok(0x7F)),
                        ZRUB1 => return Ok(ZModemResult::Ok(0xFF)),
                        ZCRCE => {
                            return Ok(ZModemResult::CrcCheckRequested(c, true, false));
                        }
                        ZCRCG => {
                            return Ok(ZModemResult::CrcCheckRequested(c, false, false));
                        }
                        ZCRCQ => {
                            return Ok(ZModemResult::CrcCheckRequested(c, false, true));
                        }
                        ZCRCW => {
                            return Ok(ZModemResult::CrcCheckRequested(c, true, true));
                        }

                        _ => {
                            // Fixed: Correct condition for dropping control chars
                            if escape_ctrl_chars && (c & 0x60) == 0 {
                                // Drop unescaped ctrl char
                                continue;
                            }

                            if c & 0x60 == 0x40 {
                                return Ok(ZModemResult::Ok(c ^ 0x40));
                            }

                            return Err(ZModemError::InvalidSubpacket(c).into());
                        }
                    }
                }
            }
            XON | XON_0X80 | XOFF | XOFF_0X80 => {
                // they should be ignored, not errored according to spec
                // log::info("ignored byte");
                continue;
            }
            _ => {
                /*
                if escape_ctrl_chars && (c & 0x60) == 0 {
                    continue;
                }
                */
                return Ok(ZModemResult::Ok(c));
            }
        }
    }
}

async fn check_crc(com: &mut dyn Connection, use_crc32: bool, data: &[u8], zcrc_byte: u8) -> crate::Result<bool> {
    if use_crc32 {
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 4).await?;
        let check_crc = u32::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            Err(ZModemError::CRC32Mismatch(crc, check_crc).into())
        }
    } else {
        let crc = get_crc16_buggy_zlde(data, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 2).await?;
        let check_crc = u16::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            Err(ZModemError::CRC16Mismatch(crc, check_crc).into())
        }
    }
}
