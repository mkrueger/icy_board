use super::{ZConnectBlock, ZConnectState};

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
                self.get(parse_mail_parameter(&parameter));
            }
            "PUT" => {
                self.put(parse_mail_parameter(&parameter));
            }
            "DEL" => {
                self.delete(parse_mail_parameter(&parameter));
            }
            "FILEREQ" => {
                self.filereq(parameter);
            }
            "FILESEND" => {
                self.filesend(&parameter);
            }
            "PGP-KEYREQ" => {
                self.pgp_keyreq();
            }
            "EXECUTE" => {
                self.execute(match parameter.as_str() {
                    "J" | "Y" => Execute::Yes, // Only 'J' is specified but in the zconnect examples 'Y' is used - so we accept both
                    "N" => Execute::No,
                    "L" => Execute::Later,
                    _ => {
                        log::error!("Unknown execute parameter: {}", parameter);
                        return Err(format!("Unknown execute parameter: {}", parameter).into());
                    }
                });
            }
            "WAIT" => {
                self.wait(match parameter.as_str() {
                    "N" => None,
                    _ => Some(parameter.parse()?),
                });
            }
            "BYTES" => {
                self.file_size(parameter.parse()?);
            }
            "FILE-CRC" => {
                self.file_crc(u32::from_str_radix(&parameter, 16)?);
            }
            "LOGOFF" => {
                self.logoff();
            }
            "RETRANSMIT" => {
                self.retransmit();
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
    /// Get mails from the server
    /// Message is for ex. 'GET:PEBF' the parameter specifies hich mail type should be requested.
    /// The server can answer with a 'PUT' message which mail types are available.
    ///
    /// If 'PUT' is empty no mails are available.
    /// Note that the mails are given in blocks so 'GET' needs to be requested multiple times until
    /// an empty 'PUT' message is received.
    pub fn get(&mut self, attr: u8) {
        self.commands.push(ZConnectCmd::Get(attr));
    }

    /// See 'GET'. 'PUT' is an answer from the mailbox to 'get'
    pub fn put(&mut self, attr: u8) {
        self.commands.push(ZConnectCmd::Put(attr));
    }

    /// Delete mails from the server with the extension '.PRV', '.KOM', '.BRT', '.ERR' or '.EIL'
    /// Routmails are never deleted. Delete has priority over 'GET'.
    pub fn delete(&mut self, attr: u8) {
        self.commands.push(ZConnectCmd::Delete(attr));
    }

    /// Optional Default: ZCONNECT
    pub fn format(&mut self) {
        self.commands.push(ZConnectCmd::Format("Z_CONNECT".to_string()));
    }

    /// Optional request file from the file server
    pub fn filereq(&mut self, path: impl Into<String>) {
        self.commands.push(ZConnectCmd::Filereq(path.into()));
    }

    /// Server wants to send file to the client
    pub fn filesend(&mut self, path: &str) {
        self.commands.push(ZConnectCmd::Filesend(path.to_string()));
    }

    /// Request PGP key from the client
    pub fn pgp_keyreq(&mut self) {
        self.commands.push(ZConnectCmd::PgpKeyreq);
    }

    pub fn execute(&mut self, exec: Execute) {
        self.commands.push(ZConnectCmd::Execute(exec));
    }

    /// Wait in seconds
    pub fn wait(&mut self, secs_opt: Option<u32>) {
        self.commands.push(ZConnectCmd::Wait(secs_opt));
    }

    /// Size of the file to transmit in bytes
    pub fn file_size(&mut self, file_size: u64) {
        self.commands.push(ZConnectCmd::Bytes(file_size));
    }

    /// Optional 32 bit CCITT/Z-MODEM crc32 of the file
    pub fn file_crc(&mut self, crc32: u32) {
        self.commands.push(ZConnectCmd::FileCrc(crc32));
    }

    /// Request logoff
    pub fn logoff(&mut self) {
        self.commands.push(ZConnectCmd::Logoff);
    }

    /// Retransmit file
    pub fn retransmit(&mut self) {
        self.commands.push(ZConnectCmd::Retransmit);
    }
}

pub enum Execute {
    Yes,
    No,
    Later,
}

#[cfg(test)]
mod tests {
    use crate::zconnect::{
        commands::{mails, ZConnectCommandBlock},
        BlockCode, ZConnectBlock,
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
        let mut blk = ZConnectCommandBlock::default();
        blk.get(mails::ALL);
        blk.put(mails::ALL);
        blk.block(BlockCode::Block1);
        assert_eq!("Get:PEBF\rPut:PEBF\rStatus:BLK1\rCRC:DC1C\r\r", blk.display());
    }
}
