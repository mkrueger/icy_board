use super::{ProtocolTransition, ZConnectBlock, ZConnectState};

pub mod mails {
    pub const ALL: u8 = PERSONAL | URGENT | NEWS | ERRORS;
    /// 'P' Personal mail
    pub const PERSONAL: u8 = 0b0000_0001;
    /// 'E' Urgent mails (German: 'Eilmails')
    pub const URGENT: u8 = 0b0000_0010;
    /// 'B' News mails (German: 'Brettnachrichten')
    pub const NEWS: u8 = 0b0000_0100;
    /// 'F' Error mails (German: 'Fehlermails')
    pub const ERRORS: u8 = 0b0000_1000;
}

pub mod system_paths {
    /// Welcome message
    pub const WELCOME: &str = "/INFO/LOGIN";
    /// Message of the day
    pub const MOTD: &str = "/INFO/MOTD";
    /// Quick Usage of the box
    pub const QUSAGE: &str = "/INFO/Q-USAGE";
    /// Verbose Usage of the box
    pub const VUSAGE: &str = "/INFO/V-USAGE";
    /// Hints & Twinkles
    pub const HINTS: &str = "/INFO/HINTS";
    /// Introduction for new users
    pub const INTRO: &str = "/INFO/INTRO";
    /// Available networks
    pub const NETWORKS: &str = "/INFO/NETWORKS";
    /// System information
    pub const SYSTEM: &str = "/INFO/SYSTEM";
    /// Login times
    pub const TIMES: &str = "/INFO/TIMES";
    /// Costs of the mailbox
    pub const COSTS: &str = "/INFO/COSTS";
    /// Code of conduct
    pub const CODE_OF_CONDUCT: &str = "/INFO/KNIGGE";
    /// File server file list
    pub const FILES: &str = "/INFO/INHALT";
}

fn get_mail_attr(attr: u8) -> String {
    let mut mail = String::new();
    if attr & mails::PERSONAL != 0 {
        mail.push_str("P");
    }
    if attr & mails::URGENT != 0 {
        mail.push_str("E");
    }
    if attr & mails::NEWS != 0 {
        mail.push_str("B");
    }
    if attr & mails::ERRORS != 0 {
        mail.push_str("F");
    }
    mail
}

pub enum ZConnectCmd {
    Get(u8),
    Put(u8),
    Delete(u8),
    Format(String),
    Filereq(String),
    Filesend(String),
    PgpKeyreq,
    Execute(Execute),
    Wait(Option<u32>),
    Bytes(u64),
    FileCrc(u32),
    Logoff,
    Retransmit,
}

impl std::fmt::Display for ZConnectCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZConnectCmd::Get(attr) => write!(f, "Get:{}", get_mail_attr(*attr)),
            ZConnectCmd::Put(attr) => write!(f, "Put:{}", get_mail_attr(*attr)),
            ZConnectCmd::Delete(attr) => write!(f, "Delete:{}", get_mail_attr(*attr)),
            ZConnectCmd::Format(s) => write!(f, "Format:{}", s),
            ZConnectCmd::Filereq(s) => write!(f, "FileReq:{}", s),
            ZConnectCmd::Filesend(s) => write!(f, "FileSend:{}", s),
            ZConnectCmd::PgpKeyreq => write!(f, "PGP-KeyReq"),
            ZConnectCmd::Execute(exec) => write!(
                f,
                "Execute:{}",
                match exec {
                    Execute::Yes => "J",
                    Execute::No => "N",
                    Execute::Later => "L",
                }
            ),
            ZConnectCmd::Wait(secs_opt) => write!(f, "Wait:{}", if let Some(seq) = secs_opt { seq.to_string() } else { "N".to_string() }),
            ZConnectCmd::Bytes(file_size) => write!(f, "Bytes:{}", file_size),
            ZConnectCmd::FileCrc(crc32) => write!(f, "File-CRC:{:08X}", crc32),
            ZConnectCmd::Logoff => write!(f, "Logoff"),
            ZConnectCmd::Retransmit => write!(f, "Retransmit"),
        }
    }
}

#[derive(Default)]
pub struct ZConnectCommandBlock {
    state: ZConnectState,
    commands: Vec<ZConnectCmd>,
}

impl ZConnectBlock for ZConnectCommandBlock {
    fn state(&self) -> ZConnectState {
        self.state
    }
    fn set_state(&mut self, state: ZConnectState) {
        self.state = state;
    }

    fn generate_lines(&self) -> Vec<String> {
        self.commands.iter().map(|cmd| cmd.to_string()).collect()
    }

    fn parse_cmd(&mut self, command: &str, parameter: String) -> crate::Result<()> {
        match command {
            "GET" => {
                self.commands.push(ZConnectCmd::Get(parse_mail_parameter(&parameter)));
            }
            "PUT" => {
                self.commands.push(ZConnectCmd::Put(parse_mail_parameter(&parameter)));
            }
            "DEL" => {
                self.commands.push(ZConnectCmd::Delete(parse_mail_parameter(&parameter)));
            }
            "FILEREQ" => {
                self.commands.push(ZConnectCmd::Filereq(parameter));
            }
            "FILESEND" => {
                self.commands.push(ZConnectCmd::Filesend(parameter));
            }
            "PGP-KEYREQ" => {
                self.commands.push(ZConnectCmd::PgpKeyreq);
            }
            "EXECUTE" => {
                self.commands.push(ZConnectCmd::Execute(Execute::from_str(&parameter)));
            }
            "WAIT" => {
                let p = match parameter.as_str() {
                    "N" => None,
                    _ => Some(parameter.parse()?),
                };
                self.commands.push(ZConnectCmd::Wait(p));
            }
            "BYTES" => {
                self.commands.push(ZConnectCmd::Bytes(parameter.parse()?));
            }
            "FILE-CRC" => {
                self.commands.push(ZConnectCmd::FileCrc(u32::from_str_radix(&parameter, 16)?));
            }
            "LOGOFF" => {
                self.commands.push(ZConnectCmd::Logoff);
            }
            "RETRANSMIT" => {
                self.commands.push(ZConnectCmd::Retransmit);
            }
            _ => {
                log::error!("Unknown zconnect command: {}", command);
            }
        }
        Ok(())
    }
}

fn parse_mail_parameter(parameter: &str) -> u8 {
    let mut res = 0;
    for ch in parameter.chars() {
        match ch {
            'P' | 'p' => res |= mails::PERSONAL,
            'E' | 'e' => res |= mails::URGENT,
            'B' | 'b' => res |= mails::NEWS,
            'F' | 'f' => res |= mails::ERRORS,
            _ => {}
        }
    }
    res
}

impl ZConnectCommandBlock {
    pub const EOT4: ZConnectCommandBlock = ZConnectCommandBlock {
        state: ZConnectState::Eot(super::EndTransmission::End4),
        commands: Vec::new(),
    };
    pub const EOT5: ZConnectCommandBlock = ZConnectCommandBlock {
        state: ZConnectState::Eot(super::EndTransmission::Prot5),
        commands: Vec::new(),
    };
    pub const BEG5: ZConnectCommandBlock = ZConnectCommandBlock {
        state: ZConnectState::Begin(ProtocolTransition::Prot5),
        commands: Vec::new(),
    };

    /// Get mails from the server
    /// Message is for ex. 'GET:PEBF' the parameter specifies hich mail type should be requested.
    /// The server can answer with a 'PUT' message which mail types are available.
    ///
    /// If 'PUT' is empty no mails are available.
    /// Note that the mails are given in blocks so 'GET' needs to be requested multiple times until
    /// an empty 'PUT' message is received.
    pub fn get(mut self, attr: u8) -> Self {
        self.commands.push(ZConnectCmd::Get(attr));
        self
    }

    /// See 'GET'. 'PUT' is an answer from the mailbox to 'get'
    pub fn put(mut self, attr: u8) -> Self {
        self.commands.push(ZConnectCmd::Put(attr));
        self
    }

    /// Delete mails from the server with the extension '.PRV', '.KOM', '.BRT', '.ERR' or '.EIL'
    /// Routmails are never deleted. Delete has priority over 'GET'.
    pub fn delete(mut self, attr: u8) -> Self {
        self.commands.push(ZConnectCmd::Delete(attr));
        self
    }

    /// Optional Default: ZCONNECT
    pub fn format(mut self) -> Self {
        self.commands.push(ZConnectCmd::Format("Z_CONNECT".to_string()));
        self
    }

    /// Optional request file from the file server
    pub fn filereq(mut self, path: impl Into<String>) -> Self {
        self.commands.push(ZConnectCmd::Filereq(path.into()));
        self
    }

    /// Server wants to send file to the client
    pub fn filesend(mut self, path: &str) -> Self {
        self.commands.push(ZConnectCmd::Filesend(path.to_string()));
        self
    }

    /// Request PGP key from the client
    pub fn pgp_keyreq(mut self) -> Self {
        self.commands.push(ZConnectCmd::PgpKeyreq);
        self
    }

    pub fn execute(mut self, exec: Execute) -> Self {
        self.commands.push(ZConnectCmd::Execute(exec));
        self
    }

    /// Wait in seconds
    pub fn wait(mut self, secs_opt: Option<u32>) -> Self {
        self.commands.push(ZConnectCmd::Wait(secs_opt));
        self
    }

    /// Size of the file to transmit in bytes
    pub fn file_size(mut self, file_size: u64) -> Self {
        self.commands.push(ZConnectCmd::Bytes(file_size));
        self
    }

    /// Optional 32 bit CCITT/Z-MODEM crc32 of the file
    pub fn file_crc(mut self, crc32: u32) -> Self {
        self.commands.push(ZConnectCmd::FileCrc(crc32));
        self
    }

    /// Request logoff
    pub fn logoff(mut self) -> Self {
        self.commands.push(ZConnectCmd::Logoff);
        self
    }

    /// Retransmit file
    pub fn retransmit(mut self) -> Self {
        self.commands.push(ZConnectCmd::Retransmit);
        self
    }

    pub fn parse(input: &str) -> crate::Result<Self> {
        let mut res = Self::default();
        res.parse_block(input)?;
        Ok(res)
    }
}

pub enum Execute {
    Yes,
    No,
    Later,
}

impl Execute {
    pub fn from_str(s: &str) -> Self {
        match s {
            "J" | "Y" => Execute::Yes,
            "N" => Execute::No,
            "L" => Execute::Later,
            _ => panic!("Unknown execute parameter: {}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::zconnect::{
        BlockCode, ZConnectBlock,
        commands::{ZConnectCommandBlock, mails},
    };

    #[test]
    fn test_crc16_single() {
        // crc taken from official documentation
        let mut blk = ZConnectCommandBlock::default();
        blk.tme(BlockCode::Block1);
        assert_eq!("Status:TME1\rCRC:F974\r\r", blk.display());
    }

    #[test]
    fn test_crc16_multiple() {
        // crc taken from another zconnect implementation.
        let mut blk = ZConnectCommandBlock::default().get(mails::ALL).put(mails::ALL);
        blk.block(BlockCode::Block1);
        assert_eq!("Get:PEBF\rPut:PEBF\rStatus:BLK1\rCRC:DC1C\r\r", blk.display());
    }
}
