use crate::crc;

pub mod header;

// const Z_CONNECT_USER: &str = "zconnect";
// const Z_CONNECT_PWD: &str = "0zconnec";

// const BEGIN : &str = "BEGIN";

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

pub enum ZConnectState {
    Block(u8),
    Ack(u8),
    Tme(u8),
    Eot(u8),
    Start(u8),
}

impl std::fmt::Display for ZConnectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZConnectState::Block(b) => write!(f, "BLK{}", *b + 1),
            ZConnectState::Ack(b) => write!(f, "ACK{}", *b + 1),
            ZConnectState::Tme(b) => write!(f, "TME{}", *b + 1),
            ZConnectState::Eot(b) => write!(f, "EOT{}", *b + 1),
            ZConnectState::Start(b) => write!(f, "BEG{}", *b + 1),
        }
    }
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
    State(ZConnectState),
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
            ZConnectCmd::State(state) => write!(f, "Status:{}", state),
        }
    }
}

#[derive(Default)]
pub struct ZConnectBlock {
    commands: Vec<ZConnectCmd>,
}

impl std::fmt::Display for ZConnectBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        let mut crc = u16::MAX;
        for cmds in &self.commands {
            let cmd_str = cmds.to_string();
            for b in cmd_str.as_bytes() {
                crc = crc::buggy_update(crc, *b);
            }
            out.push_str(&cmd_str);
            out.push('\r');
        }
        out.push_str(&format!("CRC:{:04X}\r", crc));
        write!(f, "{out}\r")
    }
}

impl ZConnectBlock {
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

    pub fn state_block(&mut self, block: u8) {
        self.commands.push(ZConnectCmd::State(ZConnectState::Block(block)));
    }

    pub fn state_ack(&mut self, block: u8) {
        self.commands.push(ZConnectCmd::State(ZConnectState::Ack(block)));
    }

    pub fn state_tme(&mut self, block: u8) {
        self.commands.push(ZConnectCmd::State(ZConnectState::Tme(block)));
    }

    pub fn state_eot(&mut self, block: u8) {
        self.commands.push(ZConnectCmd::State(ZConnectState::Eot(block)));
    }

    pub fn state_start(&mut self, block: u8) {
        self.commands.push(ZConnectCmd::State(ZConnectState::Start(block)));
    }
}

pub enum Execute {
    Yes,
    No,
    Later,
}

#[cfg(test)]
mod tests {
    use crate::zconnect::{mails, ZConnectBlock};

    #[test]
    fn test_crc16_single() {
        // crc taken from official documentation
        let mut blk = ZConnectBlock::default();
        blk.state_tme(0);
        assert_eq!("Status:TME1\rCRC:F974\r\r", blk.to_string());
    }

    #[test]
    fn test_crc16_multiple() {
        // crc taken from another zconnect implementation.
        let mut blk = ZConnectBlock::default();
        blk.get(mails::ALL);
        blk.put(mails::ALL);
        blk.state_block(0);
        assert_eq!("Get:PEBF\rPut:PEBF\rStatus:BLK1\rCRC:E756\r\r", blk.to_string());
    }
}

/*

Die Senderin wartet solange, bis die Empfängerin in ihrer Loginmeldung einen der
folgenden Strings sendet: “ogin”, “OGIN”, “ame”, “AME” und eine Übertragungspause
von einer Sekunde gefolgt ist. Sendet die angerufene MailBox nichts, schickt die Anru-
ferin nach 10 Sekunden “\r” (nur “\r”, kein “\n”, da dieses auf einigen Betriebssy-
stemen Probleme beim Einloggen verursachen kann2 ) und wartet wiederum auf die
Einloganforderung.
Bei erkannter Login-Anforderung sendet die Anruferin den Benutzernamen “zconnect\r”.
Erhält sie darauf innerhalb von 10 Sekunden keine Antwort, sendet sie “\r” und wartet
wieder auf einen der o.g. Strings. Es sollten mindestens drei Versuche gestartet werden,
bevor bei Mißerfolg die Verbindung abgebrochen wird.
Die Empfängerin antwortet bei Erhalt des Antwortstrings durch Senden der Passwor-
tabfrage, in der einer der Strings “word”, “WORD”, “wort” oder “WORT” enthalten sein
muß. Erhält die Empfängerin keine Reaktion von der Senderin, so sendet diese von sich
aus alle 2 Sekunden erneut den zuletzt gesendeten String.

*/
