use std::ops::{Deref, DerefMut};
use std::path::Path;

use super::IcyBoardSerializer;
use super::{is_false, is_true, set_true, PCBoardImport, PCBoardTextImport};
use icy_ppe::Res;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone)]
pub enum SendRecvCommand {
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

const ASC_STR: &str = "@asc";
const XMODEM_STR: &str = "@xmodem";
const XMODEMCRC_STR: &str = "@xmodemcrc";
const XMODEM1K_STR: &str = "@xmodem1k";
const XMODEM1KG_STR: &str = "@xmodem1kg";
const YMODEM_STR: &str = "@ymodem";
const YMODEMG_STR: &str = "@ymodemg";
const ZMODEM_STR: &str = "@zmodem";
const ZMODEM8K_STR: &str = "@zmodem8k";

impl<'de> Deserialize<'de> for SendRecvCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|s| {
            if s.starts_with('@') {
                match s.as_str().to_lowercase().as_str() {
                    ASC_STR => SendRecvCommand::ASCII,
                    XMODEM_STR => SendRecvCommand::XModem,
                    XMODEMCRC_STR => SendRecvCommand::XModemCRC,
                    XMODEM1K_STR => SendRecvCommand::XModem1k,
                    XMODEM1KG_STR => SendRecvCommand::XModem1kG,
                    YMODEM_STR => SendRecvCommand::YModem,
                    YMODEMG_STR => SendRecvCommand::YModemG,
                    ZMODEM_STR => SendRecvCommand::ZModem,
                    ZMODEM8K_STR => SendRecvCommand::ZModem8k,
                    _ => SendRecvCommand::ZModem,
                }
            } else {
                SendRecvCommand::External(s)
            }
        })
    }
}

impl serde::Serialize for SendRecvCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            SendRecvCommand::ASCII => ASC_STR,
            SendRecvCommand::XModem => XMODEM_STR,
            SendRecvCommand::XModemCRC => XMODEMCRC_STR,
            SendRecvCommand::XModem1k => XMODEM1K_STR,
            SendRecvCommand::XModem1kG => XMODEM1KG_STR,
            SendRecvCommand::YModem => YMODEM_STR,
            SendRecvCommand::YModemG => YMODEMG_STR,
            SendRecvCommand::ZModem => ZMODEM_STR,
            SendRecvCommand::ZModem8k => ZMODEM8K_STR,
            SendRecvCommand::External(s) => s,
        };

        s.serialize(serializer)
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Protocol {
    #[serde(rename = "enabled")]
    #[serde(skip_serializing_if = "is_true")]
    #[serde(default = "set_true")]
    pub is_enabled: bool,

    #[serde(default)]
    #[serde(rename = "batch")]
    #[serde(skip_serializing_if = "is_false")]
    pub is_batch: bool,

    #[serde(default)]
    pub char_code: char,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,

    pub send_command: SendRecvCommand,
    pub recv_command: SendRecvCommand,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SupportedProtocols {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "protocol")]
    pub protocols: Vec<Protocol>,
}

impl Deref for SupportedProtocols {
    type Target = Vec<Protocol>;
    fn deref(&self) -> &Self::Target {
        &self.protocols
    }
}

impl DerefMut for SupportedProtocols {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.protocols
    }
}

impl PCBoardTextImport for SupportedProtocols {
    fn import_data(data: String) -> Res<Self> {
        let mut res = SupportedProtocols::default();
        for line in data.lines() {
            if line.is_empty() {
                continue;
            }
            let splitted_line = line.split(',').collect::<Vec<&str>>();
            if splitted_line.len() != 7 {
                continue;
            }

            let description = splitted_line[3].to_string();
            let char_code = splitted_line[0].to_string().chars().next().unwrap_or('-');

            let (is_enabled, is_batch, command) = match char_code {
                'A' => (true, false, SendRecvCommand::ASCII),
                'X' => (true, false, SendRecvCommand::XModem),
                'C' => (true, false, SendRecvCommand::XModemCRC),
                'O' => (true, false, SendRecvCommand::XModem1k),
                'F' => (true, false, SendRecvCommand::XModem1kG),
                'Y' => (true, false, SendRecvCommand::XModem1kG),
                'G' => (true, true, SendRecvCommand::YModemG),
                'Z' => (true, true, SendRecvCommand::ZModem),
                _ => (false, true, SendRecvCommand::External("todo".to_string())),
            };

            res.protocols.push(Protocol {
                description,
                char_code,
                is_enabled,
                is_batch,
                send_command: command.clone(),
                recv_command: command,
            });
        }
        Ok(res)
    }
}

impl PCBoardImport for SupportedProtocols {
    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        PCBoardTextImport::import_pcboard(path)
    }
}

impl IcyBoardSerializer for SupportedProtocols {
    const FILE_TYPE: &'static str = "protocols";
}
