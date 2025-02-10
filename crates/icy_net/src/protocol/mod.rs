#![allow(dead_code)]
use std::{fmt::Display, path::PathBuf, str::FromStr};

pub mod xymodem;
use serde::Deserialize;
pub use xymodem::*;

pub mod zmodem;
pub use zmodem::*;

pub mod transfer_state;
use async_trait::async_trait;
pub use transfer_state::*;

use crate::Connection;

#[async_trait]
pub trait Protocol {
    async fn update_transfer(&mut self, com: &mut dyn Connection, transfer_state: &mut TransferState) -> crate::Result<()>;

    async fn initiate_send(&mut self, com: &mut dyn Connection, files: &[PathBuf]) -> crate::Result<TransferState>;

    async fn initiate_recv(&mut self, com: &mut dyn Connection) -> crate::Result<TransferState>;

    async fn cancel_transfer(&mut self, com: &mut dyn Connection) -> crate::Result<()>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TransferProtocolType {
    None,
    ASCII,
    XModem,
    XModemCRC,
    XModem1k,
    XModem1kG,
    YModem,
    YModemG,
    #[default]
    ZModem,
    ZModem8k,
    External(String),
}

#[derive(Default)]
struct Empty {}

#[async_trait]
impl Protocol for Empty {
    async fn update_transfer(&mut self, _com: &mut dyn Connection, _transfer_state: &mut TransferState) -> crate::Result<()> {
        Ok(())
    }

    async fn initiate_send(&mut self, _com: &mut dyn Connection, _files: &[PathBuf]) -> crate::Result<TransferState> {
        Ok(TransferState::new("None".to_string()))
    }

    async fn initiate_recv(&mut self, _com: &mut dyn Connection) -> crate::Result<TransferState> {
        Ok(TransferState::new("None".to_string()))
    }

    async fn cancel_transfer(&mut self, _com: &mut dyn Connection) -> crate::Result<()> {
        Ok(())
    }
}

impl TransferProtocolType {
    pub fn create(&self) -> Box<dyn Protocol> {
        match self {
            TransferProtocolType::None => Box::new(Empty::default()),
            TransferProtocolType::ASCII => Box::new(Empty::default()),
            TransferProtocolType::XModem => Box::new(XYmodem::new(XYModemVariant::XModem)),
            TransferProtocolType::XModemCRC => Box::new(XYmodem::new(XYModemVariant::XModemCRC)),
            TransferProtocolType::XModem1k => Box::new(XYmodem::new(XYModemVariant::XModem1k)),
            TransferProtocolType::XModem1kG => Box::new(XYmodem::new(XYModemVariant::XModem1kG)),
            TransferProtocolType::YModem => Box::new(XYmodem::new(XYModemVariant::YModem)),
            TransferProtocolType::YModemG => Box::new(XYmodem::new(XYModemVariant::YModemG)),
            TransferProtocolType::ZModem => Box::new(Zmodem::new(1024)),
            TransferProtocolType::ZModem8k => Box::new(Zmodem::new(8 * 1024)),
            TransferProtocolType::External(_) => Box::new(Empty::default()),
        }
    }
}

impl Display for TransferProtocolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TransferProtocolType::None => "",
            TransferProtocolType::ASCII => ASC_STR,
            TransferProtocolType::XModem => XMODEM_STR,
            TransferProtocolType::XModemCRC => XMODEMCRC_STR,
            TransferProtocolType::XModem1k => XMODEM1K_STR,
            TransferProtocolType::XModem1kG => XMODEM1KG_STR,
            TransferProtocolType::YModem => YMODEM_STR,
            TransferProtocolType::YModemG => YMODEMG_STR,
            TransferProtocolType::ZModem => ZMODEM_STR,
            TransferProtocolType::ZModem8k => ZMODEM8K_STR,
            TransferProtocolType::External(s) => s,
        };
        write!(f, "{}", s)
    }
}

impl FromStr for TransferProtocolType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = if s.starts_with('@') {
            match s.to_ascii_lowercase().as_str() {
                ASC_STR => TransferProtocolType::ASCII,
                XMODEM_STR => TransferProtocolType::XModem,
                XMODEMCRC_STR => TransferProtocolType::XModemCRC,
                XMODEM1K_STR => TransferProtocolType::XModem1k,
                XMODEM1KG_STR => TransferProtocolType::XModem1kG,
                YMODEM_STR => TransferProtocolType::YModem,
                YMODEMG_STR => TransferProtocolType::YModemG,
                ZMODEM_STR => TransferProtocolType::ZModem,
                ZMODEM8K_STR => TransferProtocolType::ZModem8k,
                _ => TransferProtocolType::None,
            }
        } else {
            TransferProtocolType::External(s.to_string())
        };
        Ok(res)
    }
}

impl From<String> for TransferProtocolType {
    fn from(s: String) -> Self {
        TransferProtocolType::from_str(&s).unwrap_or_default()
    }
}

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

pub const ASC_STR: &str = "@asc";
pub const XMODEM_STR: &str = "@xmodem";
pub const XMODEMCRC_STR: &str = "@xmodemcrc";
pub const XMODEM1K_STR: &str = "@xmodem1k";
pub const XMODEM1KG_STR: &str = "@xmodem1kg";
pub const YMODEM_STR: &str = "@ymodem";
pub const YMODEMG_STR: &str = "@ymodemg";
pub const ZMODEM_STR: &str = "@zmodem";
pub const ZMODEM8K_STR: &str = "@zmodem8k";

impl<'de> Deserialize<'de> for TransferProtocolType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(TransferProtocolType::from)
    }
}

impl serde::Serialize for TransferProtocolType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
