//
// ZModem protocol specification http://cristal.inria.fr/~doligez/zmodem/zmodem.txt

pub mod constants;

use std::path::PathBuf;

pub use constants::*;
mod headers;
pub use headers::*;

pub mod sz;
use sz::Sz;

pub mod rz;
use rz::Rz;

mod err;

use crate::{
    crc::{get_crc16_buggy_zlde, get_crc32, update_crc32},
    Connection,
};

use self::{err::ZModemError, rz::read_zdle_byte};
use super::{Protocol, TransferState};

pub struct Zmodem {
    block_length: usize,
    rz: Option<rz::Rz>,
    sz: Option<sz::Sz>,
}

impl Zmodem {
    pub fn new(block_length: usize) -> Self {
        Self {
            block_length,
            sz: None,
            rz: None,
        }
    }

    fn get_name(&self) -> &str {
        if self.block_length == 1024 {
            "Zmodem"
        } else {
            "ZedZap (Zmodem 8k)"
        }
    }

    pub fn cancel(com: &mut dyn Connection) -> crate::Result<()> {
        com.write_all(&ABORT_SEQ)?;
        Ok(())
    }

    pub fn encode_subpacket_crc16(zcrc_byte: u8, data: &[u8], escape_ctl_chars: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let crc = get_crc16_buggy_zlde(data, zcrc_byte);
        append_zdle_encoded(&mut v, data, escape_ctl_chars);

        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_zdle_encoded(&mut v, &u16::to_le_bytes(crc), escape_ctl_chars);
        v
    }

    pub fn encode_subpacket_crc32(zcrc_byte: u8, data: &[u8], escape_ctl_chars: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);

        append_zdle_encoded(&mut v, data, escape_ctl_chars);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_zdle_encoded(&mut v, &u32::to_le_bytes(crc), escape_ctl_chars);
        v
    }
}

pub fn append_zdle_encoded(v: &mut Vec<u8>, data: &[u8], escape_ctl_chars: bool) {
    let mut last = 0u8;
    for b in data {
        match *b {
            DLE | DLE_0X80 | XON | XON_0X80 | XOFF | XOFF_0X80 | ZDLE => {
                v.extend_from_slice(&[ZDLE, *b ^ 0x40]);
            }
            CR | CR_0X80 => {
                if escape_ctl_chars && last == b'@' {
                    v.extend_from_slice(&[ZDLE, *b ^ 0x40]);
                } else {
                    v.push(*b);
                }
            }

            b => {
                if escape_ctl_chars && (b & 0x60) == 0 {
                    v.extend_from_slice(&[ZDLE, b ^ 0x40]);
                } else {
                    v.push(b);
                }
            }
        }
        last = *b;
    }
}

pub fn read_zdle_bytes(com: &mut dyn Connection, length: usize) -> crate::Result<Vec<u8>> {
    let mut data = Vec::new();
    for _ in 0..length {
        let c = read_zdle_byte(com, false)?;
        if let rz::ZModemResult::Ok(b) = c {
            data.push(b);
        }
    }
    Ok(data)
}

fn get_hex(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'a' + (n - 10)
    }
}

fn from_hex(n: u8) -> crate::Result<u8> {
    if n.is_ascii_digit() {
        return Ok(n - b'0');
    }
    if (b'A'..=b'F').contains(&n) {
        return Ok(10 + n - b'A');
    }
    if (b'a'..=b'f').contains(&n) {
        return Ok(10 + n - b'a');
    }
    Err(ZModemError::HexNumberExpected.into())
}

impl Protocol for Zmodem {
    fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()> {
        if let Some(rz) = &mut self.rz {
            rz.update_transfer(com, transfer_state)?;
            if !rz.is_active() {
                transfer_state.is_finished = true;
            }
        } else if let Some(sz) = &mut self.sz {
            sz.update_transfer(com, transfer_state)?;
        }
        Ok(())
    }

    fn initiate_send(&mut self, _com: &mut dyn Connection, files: &[PathBuf]) -> crate::Result<TransferState> {
        let mut sz = Sz::new(self.block_length);
        sz.send(files);
        self.sz = Some(sz);
        Ok(TransferState::new(self.get_name().to_string()))
    }

    fn initiate_recv(&mut self, com: &mut dyn Connection) -> crate::Result<TransferState> {
        let mut rz = Rz::new(self.block_length);
        rz.recv(com)?;
        self.rz = Some(rz);
        Ok(TransferState::new(self.get_name().to_string()))
    }

    fn cancel_transfer(&mut self, com: &mut dyn Connection) -> crate::Result<()> {
        com.write_all(&ABORT_SEQ)?;
        Ok(())
    }
}
