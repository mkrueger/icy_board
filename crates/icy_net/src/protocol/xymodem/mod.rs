mod constants;
mod err;
mod ry;
mod sy;

use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::Connection;

use self::{
    constants::{CAN, CPMEOF, DEFAULT_BLOCK_LENGTH, EXT_BLOCK_LENGTH},
    err::XYModemError,
};

use super::TransferState;

#[derive(Debug, Clone, Copy)]
pub enum Checksum {
    Default,
    CRC16,
}

#[derive(Debug, Clone, Copy)]
pub enum XYModemVariant {
    /// 128 byte blocks, SOH header.
    XModem,
    /// 128 byte blocks, SOH header, CRC 16 Checksum
    XModemCRC,
    /// 1k blocks STX header, fallback to SOH for blocks <= 128 byte, CRC 16 Checksum
    XModem1k,
    /// 1k blocks STX header without ACK, fallback to SOH for blocks <= 128 byte, CRC 16 Checksum
    XModem1kG,
    /// Ymodem
    YModem,
    /// Ymodem without ACK
    YModemG,
}
impl XYModemVariant {
    fn get_block_length(&self) -> usize {
        match self {
            XYModemVariant::XModem | XYModemVariant::XModemCRC => DEFAULT_BLOCK_LENGTH,
            XYModemVariant::XModem1k | XYModemVariant::XModem1kG | XYModemVariant::YModem | XYModemVariant::YModemG => EXT_BLOCK_LENGTH,
        }
    }

    fn get_checksum_mode(&self) -> Checksum {
        match self {
            XYModemVariant::XModem => Checksum::Default,
            XYModemVariant::XModemCRC | XYModemVariant::XModem1k | XYModemVariant::XModem1kG | XYModemVariant::YModem | XYModemVariant::YModemG => {
                Checksum::CRC16
            }
        }
    }

    fn get_name(&self) -> &str {
        match self {
            XYModemVariant::XModem => "Xmodem",
            XYModemVariant::XModemCRC => "Xmodem/CRC",
            XYModemVariant::XModem1k => "Xmodem 1k",
            XYModemVariant::XModem1kG => "Xmodem 1k-G",
            XYModemVariant::YModem => "Ymodem",
            XYModemVariant::YModemG => "Ymodem-G",
        }
    }
}

/// specification: <http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt>
pub struct XYmodem {
    config: XYModemConfiguration,
    ry: Option<ry::Ry>,
    sy: Option<sy::Sy>,
}

impl XYmodem {
    pub fn new(variant: XYModemVariant) -> Self {
        XYmodem {
            config: XYModemConfiguration::new(variant),
            ry: None,
            sy: None,
        }
    }
}

impl super::Protocol for XYmodem {
    fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let Some(ry) = &mut self.ry {
            ry.update_transfer(com, transfer_state)?;
            transfer_state.is_finished = ry.is_finished();
        } else if let Some(sy) = &mut self.sy {
            sy.update_transfer(com, transfer_state)?;
            transfer_state.is_finished = sy.is_finished();
        }
        Ok(())
    }

    fn initiate_send(&mut self, _com: &mut dyn Connection, files: &[PathBuf]) -> crate::Result<TransferState> {
        if !self.config.is_ymodem() && files.len() != 1 {
            return Err(XYModemError::XModem1File.into());
        }

        let mut sy = sy::Sy::new(self.config);
        sy.send(files);
        self.sy = Some(sy);
        Ok(TransferState::new(self.config.get_protocol_name().to_string()))
    }

    fn initiate_recv(&mut self, com: &mut dyn Connection) -> crate::Result<TransferState> {
        let mut ry = ry::Ry::new(self.config);
        ry.recv(com)?;
        self.ry = Some(ry);

        Ok(TransferState::new(self.config.get_protocol_name().to_string()))
    }

    fn cancel_transfer(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        cancel(com)
    }
}

fn cancel(com: &mut dyn Connection) -> crate::Result<()> {
    com.write_all(&[CAN, CAN, CAN, CAN, CAN, CAN])?;
    Ok(())
}

fn get_checksum(block: &[u8]) -> u8 {
    block.iter().fold(0, |x, &y| x.wrapping_add(y))
}

#[derive(Clone, Copy)]
pub struct XYModemConfiguration {
    pub variant: XYModemVariant,
    pub block_length: usize,
    pub checksum_mode: Checksum,
}

impl XYModemConfiguration {
    fn new(variant: XYModemVariant) -> Self {
        let block_length = variant.get_block_length();
        let checksum_mode = variant.get_checksum_mode();

        Self {
            variant,
            block_length,
            checksum_mode,
        }
    }

    fn get_protocol_name(&self) -> &str {
        self.variant.get_name()
    }

    fn get_check_and_size(&self) -> String {
        let checksum = if let Checksum::Default = self.checksum_mode { "Checksum" } else { "Crc" };
        let block = if self.block_length == DEFAULT_BLOCK_LENGTH { "128" } else { "1k" };
        format!("{checksum}/{block}")
    }

    fn is_ymodem(&self) -> bool {
        matches!(self.variant, XYModemVariant::YModem | XYModemVariant::YModemG)
    }

    fn is_streaming(&self) -> bool {
        matches!(self.variant, XYModemVariant::XModem1kG | XYModemVariant::YModemG)
    }

    fn use_crc(&self) -> bool {
        match self.checksum_mode {
            Checksum::CRC16 => true,
            Checksum::Default => false,
        }
    }
}

pub fn remove_cpm_eof<P: AsRef<Path>>(file_name: P, max_len: usize) -> crate::Result<u64> {
    let mut buf = vec![0; max_len];

    let new_len = {
        let mut f = File::open(&file_name)?;
        let mut bytes_left = f.metadata()?.len();
        let read_bytes = (buf.len() as u64).min(bytes_left);
        f.seek(SeekFrom::End(-(read_bytes as i64)))?;

        let read_bytes = f.read(&mut buf)?;
        for i in (0..read_bytes).rev() {
            if buf[i] != CPMEOF {
                break;
            }
            bytes_left -= 1;
        }
        bytes_left
    };
    let f = OpenOptions::new().write(true).open(&file_name)?;
    f.set_len(new_len)?;
    Ok(new_len)
}
