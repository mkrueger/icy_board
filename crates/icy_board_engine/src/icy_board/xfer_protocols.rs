use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use super::IcyBoardSerializer;
use super::{PCBoardImport, PCBoardTextImport, is_false, is_true, set_true};
use crate::Res;
use icy_net::protocol::TransferProtocolType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
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
    #[serde(rename = "bidirectional")]
    #[serde(skip_serializing_if = "is_false")]
    pub is_bi_directional: bool,

    #[serde(default)]
    pub char_code: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,

    pub send_command: TransferProtocolType,
    pub recv_command: TransferProtocolType,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SupportedProtocols {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "protocol")]
    protocols: Vec<Protocol>,
}

impl SupportedProtocols {
    pub fn export_data(&self, output: &Path) -> Res<()> {
        let mut res = String::new();
        for protocol in &self.protocols {
            let char_code = protocol.char_code.chars().next().unwrap_or('-');
            // I internal
            // S shelled
            // D shelled + DSZLOG for batch transfer
            // B shelled + bidirectional + DSZLOG
            let prot_type = "I";

            let block_size = 1024;
            let description = &protocol.description;

            let mnp = "N"; // error corrected session
            let port_open = "N"; // leave port open during shell
            let lock_lines = "N"; // lock status lines on screen
            res.push_str(&format!(
                "{},{},{},{},{},{},{}\r\n",
                char_code, prot_type, block_size, description, mnp, port_open, lock_lines
            ));
        }
        fs::write(output, res)?;
        Ok(())
    }

    pub fn find_protocol(&self, char_code: &str) -> Option<&Protocol> {
        for p in &self.protocols {
            if p.char_code == char_code {
                return Some(p);
            }
        }
        None
    }

    /// Generate a default set of protocols
    pub fn generate_pcboard_defaults() -> Self {
        let mut protocols = Vec::new();
        protocols.push(Protocol {
            is_enabled: true,
            is_batch: false,
            is_bi_directional: false,
            char_code: "A".to_string(),
            description: "Ascii".to_string(),
            send_command: TransferProtocolType::ASCII,
            recv_command: TransferProtocolType::ASCII,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: false,
            is_bi_directional: false,
            char_code: "X".to_string(),
            description: "Xmodem/Checksum".to_string(),
            send_command: TransferProtocolType::XModem,
            recv_command: TransferProtocolType::XModem,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: false,
            is_bi_directional: false,
            char_code: "C".to_string(),
            description: "Xmodem/CRC".to_string(),
            send_command: TransferProtocolType::XModemCRC,
            recv_command: TransferProtocolType::XModemCRC,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: false,
            is_bi_directional: false,
            char_code: "O".to_string(),
            description: "1K-Xmodem       (a.k.a. non-BATCH Ymodem)".to_string(),
            send_command: TransferProtocolType::XModem1k,
            recv_command: TransferProtocolType::XModem1k,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: false,
            is_bi_directional: false,
            char_code: "F".to_string(),
            description: "1K-Xmodem/G     (a.k.a. non-BATCH Ymodem/G)".to_string(),
            send_command: TransferProtocolType::XModem1kG,
            recv_command: TransferProtocolType::XModem1kG,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: true,
            is_bi_directional: false,
            char_code: "Y".to_string(),
            description: "Ymodem BATCH".to_string(),
            send_command: TransferProtocolType::YModem,
            recv_command: TransferProtocolType::YModem,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: true,
            is_bi_directional: false,
            char_code: "G".to_string(),
            description: "Ymodem/G BATCH".to_string(),
            send_command: TransferProtocolType::YModemG,
            recv_command: TransferProtocolType::YModemG,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: true,
            is_bi_directional: false,
            char_code: "Z".to_string(),
            description: "Zmodem (batch)".to_string(),
            send_command: TransferProtocolType::ZModem,
            recv_command: TransferProtocolType::ZModem,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: true,
            is_bi_directional: false,
            char_code: "8".to_string(),
            description: "Zmodem 8k (batch)".to_string(),
            send_command: TransferProtocolType::ZModem8k,
            recv_command: TransferProtocolType::ZModem8k,
        });

        protocols.push(Protocol {
            is_enabled: true,
            is_batch: true,
            is_bi_directional: false,
            char_code: "N".to_string(),
            description: "None".to_string(),
            send_command: TransferProtocolType::None,
            recv_command: TransferProtocolType::None,
        });
        Self { protocols }
    }
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
            let is_bi_directional = false;
            res.protocols.push(Protocol {
                description,
                char_code: char_code.to_string(),
                is_enabled,
                is_batch,
                is_bi_directional,
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
