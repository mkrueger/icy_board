#![allow(dead_code)]
use std::path::PathBuf;

pub mod xymodem;
pub use xymodem::*;

pub mod zmodem;
pub use zmodem::*;

pub mod transfer_state;
pub use transfer_state::*;

use crate::Connection;

pub trait Protocol {
    fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()>;

    fn initiate_send(&mut self, com: &mut dyn Connection, files: &[PathBuf]) -> crate::Result<TransferState>;

    fn initiate_recv(&mut self, com: &mut dyn Connection) -> crate::Result<TransferState>;

    fn cancel_transfer(&mut self, com: &mut dyn Connection) -> crate::Result<()>;
}

/*
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TransferType {
    #[default]
    ZModem,
    ZedZap,
    XModem,
    XModem1k,
    XModem1kG,
    YModem,
    YModemG,
    Text,
}

impl TransferType {
    pub fn create(self) -> Box<dyn Protocol> {
        match self {
            TransferType::ZModem => Box::new(Zmodem::new(1024)),
            TransferType::ZedZap => Box::new(Zmodem::new(8 * 1024)),
            TransferType::XModem => Box::new(XYmodem::new(XYModemVariant::XModem)),
            TransferType::XModem1k => Box::new(XYmodem::new(XYModemVariant::XModem1k)),
            TransferType::XModem1kG => Box::new(XYmodem::new(XYModemVariant::XModem1kG)),
            TransferType::YModem => Box::new(XYmodem::new(XYModemVariant::YModem)),
            TransferType::YModemG => Box::new(XYmodem::new(XYModemVariant::YModemG)),
            TransferType::Text => panic!("Not implemented"),
        }
    }
}
*/

pub fn str_from_null_terminated_utf8_unchecked(s: &[u8]) -> String {
    let mut res = String::new();
    for b in s {
        if *b == 0 {
            break;
        }
        res.push(*b as char);
    }
    res
}
