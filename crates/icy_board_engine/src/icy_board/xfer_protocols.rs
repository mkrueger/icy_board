use std::ops::{Deref, DerefMut};
use std::path::Path;

use super::IcyBoardSerializer;
use super::{is_false, is_true, set_true, PCBoardImport, PCBoardTextImport};
use crate::Res;
use icy_net::protocol::TransferProtocolType;
use serde::{Deserialize, Serialize};

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
    pub char_code: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,

    pub send_command: TransferProtocolType,
    pub recv_command: TransferProtocolType,
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
                'A' => (true, false, TransferProtocolType::ASCII),
                'X' => (true, false, TransferProtocolType::XModem),
                'C' => (true, false, TransferProtocolType::XModemCRC),
                'O' => (true, false, TransferProtocolType::XModem1k),
                'F' => (true, false, TransferProtocolType::XModem1kG),
                'Y' => (true, false, TransferProtocolType::XModem1kG),
                'G' => (true, true, TransferProtocolType::YModemG),
                'Z' => (true, true, TransferProtocolType::ZModem),
                _ => (false, true, TransferProtocolType::External("todo".to_string())),
            };

            res.protocols.push(Protocol {
                description,
                char_code: char_code.to_string(),
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
