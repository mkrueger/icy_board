use std::collections::HashMap;

use super::{ZConnectBlock, ZConnectState};

#[derive(Clone, Debug, PartialEq)]
pub enum Acer {
    /// ARC/PAK
    Arc,
    /// Arj
    Arj,
    /// LH1/2
    LHArc,
    /// LH 1-5
    LHA,
    /// RAR
    RAR,
    /// ZOO
    ZOO,
    /// ZIP
    ZIP,
    /// ZIP V2.0
    ZIP2,
    /// Compress
    Compress,
    /// GNU-Zip
    GZIP,
    /// Tar und Compress
    TarCompress,
    /// Tar und GNU-Zip
    TarGZip,
    /// Remove
    RM,
    /// None
    NONE,

    Unknown(String),
}

impl Acer {
    fn parse(s: &str) -> Acer {
        match s.to_ascii_uppercase().as_str() {
            "ARC" => Acer::Arc,
            "ARJ" => Acer::Arj,
            "LHARC" | "LZH" => Acer::LHArc,
            "LHA" => Acer::LHA,
            "RAR" => Acer::RAR,
            "ZOO" => Acer::ZOO,
            "ZIP" | "ZI1" => Acer::ZIP,
            "ZIP2" | "ZI2" => Acer::ZIP2,
            "COMPRESS" => Acer::Compress,
            "GZIP" => Acer::GZIP,
            "TAR-COMPRESS" => Acer::TarCompress,
            "TAR-GZIP" => Acer::TarGZip,
            "RM" => Acer::RM,
            "NONE" => Acer::NONE,
            _ => Acer::Unknown(s.to_string()),
        }
    }
}

impl std::fmt::Display for Acer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Acer::Arc => write!(f, "ARC"),
            Acer::Arj => write!(f, "ARJ"),
            Acer::LHArc => write!(f, "LHARC"),
            Acer::LHA => write!(f, "LHA"),
            Acer::RAR => write!(f, "RAR"),
            Acer::ZOO => write!(f, "ZOO"),
            Acer::ZIP => write!(f, "ZIP"),
            Acer::ZIP2 => write!(f, "ZIP2"),
            Acer::Compress => write!(f, "COMPRESS"),
            Acer::GZIP => write!(f, "GZIP"),
            Acer::TarCompress => write!(f, "TAR-COMPRESS"),
            Acer::TarGZip => write!(f, "TAR-GZIP"),
            Acer::RM => write!(f, "RM"),
            Acer::NONE => write!(f, "NONE"),
            Acer::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mailer {
    ZConnect,
    ZConnect3,
    ZConnect31,
    Znetz,
    FTS0001,
    FSC0056,
    MausTausch,
    Unknown(String),
}
impl Mailer {
    fn parse(s: &str) -> Mailer {
        match s.to_ascii_uppercase().as_str() {
            "ZCONNECT" => Mailer::ZConnect,
            "ZCONNECT3.0" => Mailer::ZConnect3,
            "ZCONNECT3.1" => Mailer::ZConnect31,
            "ZNETZ" => Mailer::Znetz,
            "FTS0001" => Mailer::FTS0001,
            "FSC0056" => Mailer::FSC0056,
            "MAUSTAUSCH" => Mailer::MausTausch,
            _ => Mailer::Unknown(s.to_string()),
        }
    }
}

impl std::fmt::Display for Mailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mailer::ZConnect => write!(f, "ZCONNECT"),
            Mailer::ZConnect3 => write!(f, "ZCONNECT3.0"),
            Mailer::ZConnect31 => write!(f, "ZCONNECT3.1"),
            Mailer::Znetz => write!(f, "ZNETZ"),
            Mailer::FTS0001 => write!(f, "FTS0001"),
            Mailer::FSC0056 => write!(f, "FSC0056"),
            Mailer::MausTausch => write!(f, "MausTausch"),
            Mailer::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mailformat {
    ZConnect,
    ZConnect3,
    ZConnect31,
    Znetz,
    Rfc1036,
    X400,
    Unknown(String),
}

impl Mailformat {
    fn parse(s: &str) -> Mailformat {
        match s.to_ascii_uppercase().as_str() {
            "ZCONNECT" => Mailformat::ZConnect,
            "ZCONNECT3.0" => Mailformat::ZConnect3,
            "ZCONNECT3.1" => Mailformat::ZConnect31,
            "ZNETZ" => Mailformat::Znetz,
            "RFC1036" => Mailformat::Rfc1036,
            "X400" => Mailformat::X400,
            _ => Mailformat::Unknown(s.to_string()),
        }
    }
}

impl std::fmt::Display for Mailformat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mailformat::ZConnect => write!(f, "ZCONNECT"),
            Mailformat::ZConnect3 => write!(f, "ZCONNECT3.0"),
            Mailformat::ZConnect31 => write!(f, "ZCONNECT3.1"),
            Mailformat::Znetz => write!(f, "ZNETZ"),
            Mailformat::Rfc1036 => write!(f, "RFC1036"),
            Mailformat::X400 => write!(f, "X400"),
            Mailformat::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransferProtocol {
    XModem,
    YModem,
    ZModem,
    SeaLink,
    Kermit,
    BiModem,
    HSLink,
    ACopy,
    Hydra,
    EFT,
    ZModem8k,
    NCopy,
    Unknown(String),
}

impl TransferProtocol {
    fn parse(s: &str) -> TransferProtocol {
        match s.to_ascii_uppercase().as_str() {
            "XMODEM" => TransferProtocol::XModem,
            "YMODEM" => TransferProtocol::YModem,
            "ZMODEM" => TransferProtocol::ZModem,
            "SEALINK" => TransferProtocol::SeaLink,
            "KERMIT-B" => TransferProtocol::Kermit,
            "BIMODEM" => TransferProtocol::BiModem,
            "HSLINK" => TransferProtocol::HSLink,
            "ACOPY" => TransferProtocol::ACopy,
            "HYDRA" => TransferProtocol::Hydra,
            "EFT" => TransferProtocol::EFT,
            "ZMODEM8K" => TransferProtocol::ZModem8k,
            "NCOPY" => TransferProtocol::NCopy,
            _ => TransferProtocol::Unknown(s.to_string()),
        }
    }
}

impl std::fmt::Display for TransferProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferProtocol::XModem => write!(f, "XMODEM"),
            TransferProtocol::YModem => write!(f, "YMODEM"),
            TransferProtocol::ZModem => write!(f, "ZMODEM"),
            TransferProtocol::SeaLink => write!(f, "SEALINK"),
            TransferProtocol::Kermit => write!(f, "KERMIT-B"),
            TransferProtocol::BiModem => write!(f, "BIMODEM"),
            TransferProtocol::HSLink => write!(f, "HSLINK"),
            TransferProtocol::ACopy => write!(f, "ACOPY"),
            TransferProtocol::Hydra => write!(f, "HYDRA"),
            TransferProtocol::EFT => write!(f, "EFT"),
            TransferProtocol::ZModem8k => write!(f, "ZMODEM8K"),
            TransferProtocol::NCopy => write!(f, "NCOPY"),
            TransferProtocol::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Crypt {
    DES,
    PGP,
    Unknown(String),
}

impl Crypt {
    fn parse(s: &str) -> Crypt {
        match s.to_ascii_uppercase().as_str() {
            "DES" => Crypt::DES,
            "PGP" => Crypt::PGP,
            _ => Crypt::Unknown(s.to_string()),
        }
    }
}

impl std::fmt::Display for Crypt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Crypt::DES => write!(f, "DES"),
            Crypt::PGP => write!(f, "PGP"),
            Crypt::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Default)]
pub struct ZConnectHeaderBlock {
    state: ZConnectState,
    system: String,
    sysop: String,
    serial: Option<String>,
    post: Option<String>,
    port: usize,
    phone: HashMap<usize, String>,
    domains: Vec<String>,
    maps: Option<String>,
    iso2: HashMap<usize, String>,
    iso3: HashMap<usize, String>,
    acer: HashMap<usize, Vec<Acer>>,
    password: String,
    protocols: HashMap<usize, Vec<TransferProtocol>>,
    voice_phone: Option<String>,
    crypt: HashMap<usize, Vec<Crypt>>,
    acer_in: Option<Acer>,
    acer_out: Option<Acer>,
    mailer: HashMap<usize, Vec<Mailer>>,
    mailformat: HashMap<usize, Vec<Mailformat>>, // Not sure about position
    coords: Option<String>,                      // Not sure about position
}

impl ZConnectHeaderBlock {
    pub fn system(&self) -> &str {
        &self.system
    }
    pub fn set_system(&mut self, system: impl Into<String>) {
        self.system = system.into();
    }

    pub fn sysop(&self) -> &str {
        &self.sysop
    }
    pub fn set_sysop(&mut self, sysop: impl Into<String>) {
        self.sysop = sysop.into();
    }

    pub fn serial(&self) -> Option<&str> {
        self.serial.as_deref()
    }
    pub fn set_serial(&mut self, serial: impl Into<String>) {
        self.serial = Some(serial.into());
    }

    pub fn post(&self) -> Option<&str> {
        self.post.as_deref()
    }
    pub fn set_post(&mut self, post: impl Into<String>) {
        self.post = Some(post.into());
    }

    pub fn port(&self) -> usize {
        self.port
    }
    pub fn set_port(&mut self, port: usize) {
        self.port = port;
    }

    pub fn phone(&self) -> &HashMap<usize, String> {
        &self.phone
    }
    pub fn add_phone(&mut self, index: usize, phone: impl Into<String>) {
        self.phone.insert(index, phone.into());
    }

    pub fn domains(&self) -> &Vec<String> {
        &self.domains
    }
    pub fn set_domains(&mut self, domains: Vec<String>) {
        self.domains = domains;
    }

    pub fn set_maps(&mut self, maps: impl Into<String>) {
        self.maps = Some(maps.into());
    }

    pub fn iso2(&self) -> &HashMap<usize, String> {
        &self.iso2
    }
    pub fn add_iso2(&mut self, index: usize, iso2: impl Into<String>) {
        self.iso2.insert(index, iso2.into());
    }

    pub fn iso3(&self) -> &HashMap<usize, String> {
        &self.iso3
    }
    pub fn add_iso3(&mut self, index: usize, iso3: impl Into<String>) {
        self.iso3.insert(index, iso3.into());
    }

    pub fn acer(&self, index: usize) -> Option<&Vec<Acer>> {
        self.acer.get(&index)
    }

    pub fn add_acer(&mut self, index: usize, compressor: Acer) {
        self.acer.entry(index).or_insert_with(Vec::new).push(compressor);
    }

    pub fn password(&self) -> &str {
        &self.password
    }
    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = password.into();
    }

    pub fn voice_phone(&self) -> Option<&str> {
        self.voice_phone.as_deref()
    }
    pub fn set_voice_phone(&mut self, voice_phone: impl Into<String>) {
        self.voice_phone = Some(voice_phone.into());
    }

    pub fn protocols(&self, index: usize) -> Option<&Vec<TransferProtocol>> {
        self.protocols.get(&index)
    }
    pub fn add_protocol(&mut self, index: usize, protocol: TransferProtocol) {
        self.protocols.entry(index).or_insert_with(Vec::new).push(protocol);
    }

    pub fn crypt(&self, index: usize) -> Option<&Vec<Crypt>> {
        self.crypt.get(&index)
    }
    pub fn add_crypt(&mut self, index: usize, crypt: Crypt) {
        self.crypt.entry(index).or_insert_with(Vec::new).push(crypt);
    }

    pub fn acer_in(&self) -> Option<Acer> {
        self.acer_in.clone()
    }
    pub fn set_acer_in(&mut self, acer_in: Acer) {
        self.acer_in = Some(acer_in);
    }

    pub fn acer_out(&self) -> Option<Acer> {
        self.acer_out.clone()
    }
    pub fn set_acer_out(&mut self, acer_out: Acer) {
        self.acer_out = Some(acer_out);
    }

    pub fn mailer(&self, index: usize) -> Option<&Vec<Mailer>> {
        self.mailer.get(&index)
    }
    pub fn add_mailer(&mut self, index: usize, mailer: Mailer) {
        self.mailer.entry(index).or_insert_with(Vec::new).push(mailer);
    }

    pub fn mailformat(&self, index: usize) -> Option<&Vec<Mailformat>> {
        self.mailformat.get(&index)
    }
    pub fn add_mailformat(&mut self, index: usize, mailformat: Mailformat) {
        self.mailformat.entry(index).or_insert_with(Vec::new).push(mailformat);
    }

    pub fn coords(&self) -> Option<&str> {
        self.coords.as_deref()
    }
    pub fn set_coords(&mut self, coords: impl Into<String>) {
        self.coords = Some(coords.into());
    }

    pub fn parse(input: &str) -> crate::Result<Self> {
        let mut res = Self::default();
        res.parse_block(input)?;
        Ok(res)
    }
}

impl ZConnectBlock for ZConnectHeaderBlock {
    fn state(&self) -> ZConnectState {
        self.state
    }
    fn set_state(&mut self, state: ZConnectState) {
        self.state = state;
    }

    fn generate_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("Sys:{}", self.system));
        lines.push(format!("Sysop:{}", self.sysop));
        if let Some(serial) = &self.serial {
            lines.push(format!("SerNr:{}", serial));
        }
        if let Some(post) = &self.post {
            lines.push(format!("Post:{}", post));
        }
        lines.push(format!("Port:{}", self.port + 1));

        for (i, p) in self.phone.iter() {
            lines.push(format!("Tel:{} {}", i + 1, p));
        }
        lines.push(format!("Domain:{}", self.domains.join(";")));
        if let Some(maps) = &self.maps {
            lines.push(format!("Maps:{}", maps));
        }

        for (i, iso2) in self.iso2.iter() {
            lines.push(format!("ISO2:{} {}", i + 1, iso2));
        }

        for (i, iso3) in self.iso3.iter() {
            lines.push(format!("ISO3:{} {}", i + 1, iso3));
        }

        for (i, comp) in self.acer.iter() {
            lines.push(format!(
                "Arc:{} {}",
                i + 1,
                comp.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(";")
            ));
        }

        for (i, prot) in self.protocols.iter() {
            lines.push(format!(
                "Proto:{} {}",
                i + 1,
                prot.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(";")
            ));
        }

        if !self.password.is_empty() {
            lines.push(format!("Passwd:{}", self.password));
        }

        if let Some(voice_phone) = &self.voice_phone {
            lines.push(format!("Telefon:{}", voice_phone));
        }

        for (i, crypt) in self.crypt.iter() {
            lines.push(format!(
                "Crypt:{} {}",
                i + 1,
                crypt.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(";")
            ));
        }

        if let Some(acer_in) = &self.acer_in {
            lines.push(format!("Acerin:{}", acer_in));
        }

        if let Some(acer_out) = &self.acer_out {
            lines.push(format!("Acerout:{}", acer_out));
        }

        for (i, mailer) in self.mailer.iter() {
            lines.push(format!(
                "Mailer:{} {}",
                i + 1,
                mailer.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(";")
            ));
        }

        for (i, mailformat) in self.mailformat.iter() {
            lines.push(format!(
                "Mailformat:{} {}",
                i + 1,
                mailformat.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(";")
            ));
        }

        if let Some(coords) = &self.coords {
            lines.push(format!("Koordinaten:{}", coords));
        }
        lines
    }

    fn parse_cmd(&mut self, command: &str, parameter: String) -> crate::Result<()> {
        match command {
            "SYS" => self.system = parameter,
            "SYSOP" => self.sysop = parameter,
            "SERNR" => self.serial = Some(parameter),
            "POST" => self.post = Some(parameter),
            "PORT" => self.port = parameter.parse().unwrap_or(1) - 1,
            "TEL" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let phone = parts.next().unwrap().to_string();
                self.phone.insert(index - 1, phone);
            }
            "DOMAIN" => self.domains = parameter.split(|c| c == ';' || c == ' ').map(|s| s.to_string()).collect(),
            "MAPS" => self.maps = Some(parameter),
            "ISO2" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let iso2 = parts.next().unwrap().to_string();
                self.iso2.insert(index - 1, iso2);
            }
            "ISO3" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let iso3 = parts.next().unwrap().to_string();
                self.iso3.insert(index - 1, iso3);
            }
            "ARC" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let compressors = parts.next().unwrap().split(|c| c == ';' || c == ' ').map(|s| Acer::parse(s)).collect();
                self.acer.insert(index - 1, compressors);
            }
            "PASSWD" => self.password = parameter,
            "TELEFON" => self.voice_phone = Some(parameter),
            "PROTO" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let protocols = parts
                    .next()
                    .unwrap()
                    .split(|c| c == ';' || c == ' ')
                    .map(|s| TransferProtocol::parse(s))
                    .collect();
                self.protocols.insert(index - 1, protocols);
            }
            "CRYPT" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let crypts = parts.next().unwrap().split(|c| c == ';' || c == ' ').map(|s| Crypt::parse(s)).collect();
                self.crypt.insert(index - 1, crypts);
            }
            "ACERIN" => self.acer_in = Some(Acer::parse(&parameter)),
            "ACEROUT" => self.acer_out = Some(Acer::parse(&parameter)),
            "MAILER" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let mailers = parts.next().unwrap().split(|c| c == ';' || c == ' ').map(|s| Mailer::parse(s)).collect();
                self.mailer.insert(index - 1, mailers);
            }
            "MAILFORMAT" => {
                let mut parts = parameter.splitn(2, ' ');
                let index: usize = parts.next().unwrap().parse()?;
                let mailformats = parts.next().unwrap().split(|c| c == ';' || c == ' ').map(|s| Mailformat::parse(s)).collect();
                self.mailformat.insert(index - 1, mailformats);
            }
            "KOORDINATEN" => self.coords = Some(parameter),
            _ => {
                log::warn!("unknown Z-connect header entry: {}", command)
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::zconnect::{ZConnectBlock, header::ZConnectHeaderBlock};

    #[test]
    fn test_generate_header() {
        // crc taken from official documentation
        let mut blk = ZConnectHeaderBlock::default();
        blk.set_system("Icy Shadow BBS");
        blk.set_sysop("SYSOP");
        blk.set_serial("123456");
        blk.set_post("Zossen, Germany");
        blk.set_port(0);
        blk.add_phone(0, "unknown");
        blk.set_domains(vec!["icyshadow.com".to_string()]);
        blk.add_iso2(0, "V.32bis");
        blk.add_iso3(0, "V.42.bis");
        blk.add_acer(0, crate::zconnect::header::Acer::ZIP);
        blk.add_acer(0, crate::zconnect::header::Acer::Arj);
        blk.set_password("password");
        blk.set_voice_phone("+49-VOICE");
        blk.add_protocol(0, crate::zconnect::header::TransferProtocol::ZModem);
        blk.set_acer_in(crate::zconnect::header::Acer::ZIP);
        blk.set_acer_out(crate::zconnect::header::Acer::ZIP);
        blk.add_mailer(0, crate::zconnect::header::Mailer::ZConnect);
        assert_eq!(
            "Sys:Icy Shadow BBS\rSysop:SYSOP\rSerNr:123456\rPost:Zossen, Germany\rPort:1\rTel:1 unknown\rDomain:icyshadow.com\rISO2:1 V.32bis\rISO3:1 V.42.bis\rArc:1 ZIP;ARJ\rProto:1 ZMODEM\rPasswd:password\rTelefon:+49-VOICE\rAcerin:ZIP\rAcerout:ZIP\rMailer:1 ZCONNECT\rStatus:BLK1\rCRC:1095\r\r",
            blk.display()
        );
    }

    #[test]
    fn test_parse_header() {
        let header = ZConnectHeaderBlock::parse("Sys:Icy Shadow BBS\rSysop:SYSOP\rSerNr:123456\rPost:Zossen, Germany\rPort:1\rTel:1 unknown\rDomain:icyshadow.com\rISO2:1 V.32bis\rISO3:1 V.42.bis\rArc:1 ZIP;ARJ\rProto:1 ZMODEM\rPasswd:password\rTelefon:+49-VOICE\rAcerin:ZIP\rAcerout:ZIP\rMailer:1 ZCONNECT\rStatus:BLK1\rCRC:1095\r\r").unwrap();
        assert_eq!(header.system(), "Icy Shadow BBS");
        assert_eq!(header.sysop(), "SYSOP");
        assert_eq!(header.serial(), Some("123456"));
        assert_eq!(header.post(), Some("Zossen, Germany"));
        assert_eq!(header.port(), 0);
        assert_eq!(header.phone().get(&0), Some(&"unknown".to_string()));
        assert_eq!(header.domains(), &vec!["icyshadow.com".to_string()]);
        assert_eq!(header.iso2().get(&0), Some(&"V.32bis".to_string()));
        assert_eq!(header.iso3().get(&0), Some(&"V.42.bis".to_string()));
        assert_eq!(
            header.acer(0),
            Some(&vec![crate::zconnect::header::Acer::ZIP, crate::zconnect::header::Acer::Arj])
        );
        assert_eq!(header.password(), "password");
        assert_eq!(header.voice_phone(), Some("+49-VOICE"));
        assert_eq!(header.protocols(0), Some(&vec![crate::zconnect::header::TransferProtocol::ZModem]));
        assert_eq!(header.acer_in(), Some(crate::zconnect::header::Acer::ZIP));
        assert_eq!(header.acer_out(), Some(crate::zconnect::header::Acer::ZIP));
        assert_eq!(header.mailer(0), Some(&vec![crate::zconnect::header::Mailer::ZConnect]));
    }

    #[test]
    fn test_crc_position() {
        let header = ZConnectHeaderBlock::parse("CRC:1095\rSys:Icy Shadow BBS\rSysop:SYSOP\rSerNr:123456\rPost:Zossen, Germany\rPort:1\rTel:1 unknown\rDomain:icyshadow.com\rISO2:1 V.32bis\rISO3:1 V.42.bis\rArc:1 ZIP;ARJ\rProto:1 ZMODEM\rPasswd:password\rTelefon:+49-VOICE\rAcerin:ZIP\rAcerout:ZIP\rMailer:1 ZCONNECT\rStatus:BLK1\r\r").unwrap();
        assert_eq!(header.system(), "Icy Shadow BBS");
        assert_eq!(header.sysop(), "SYSOP");
        assert_eq!(header.serial(), Some("123456"));
        assert_eq!(header.post(), Some("Zossen, Germany"));
        assert_eq!(header.port(), 0);
        assert_eq!(header.phone().get(&0), Some(&"unknown".to_string()));
        assert_eq!(header.domains(), &vec!["icyshadow.com".to_string()]);
        assert_eq!(header.iso2().get(&0), Some(&"V.32bis".to_string()));
        assert_eq!(header.iso3().get(&0), Some(&"V.42.bis".to_string()));
        assert_eq!(
            header.acer(0),
            Some(&vec![crate::zconnect::header::Acer::ZIP, crate::zconnect::header::Acer::Arj])
        );
        assert_eq!(header.password(), "password");
        assert_eq!(header.voice_phone(), Some("+49-VOICE"));
        assert_eq!(header.protocols(0), Some(&vec![crate::zconnect::header::TransferProtocol::ZModem]));
        assert_eq!(header.acer_in(), Some(crate::zconnect::header::Acer::ZIP));
        assert_eq!(header.acer_out(), Some(crate::zconnect::header::Acer::ZIP));
        assert_eq!(header.mailer(0), Some(&vec![crate::zconnect::header::Mailer::ZConnect]));
    }

    #[test]
    fn test_crc_mismatch() {
        let header = ZConnectHeaderBlock::parse(
            "Sys:Icy Shadow BBS\rSysop:SYSOP\rSerNr:123456\rPost:Zossen, Germany\rPort:1\rTel:1 223\rDomain:icyshadow.com\rISO2:1 V.32bis\rISO3:1 V.42.bis\rArc:1 ZIP;ARJ\rProto:1 ZMODEM\rPasswd:password\rTelefon:+49-VOICE\rStatus:BLK1\rAcerIn:ZIP\rAcerOut:ZIP\rMailer:1 ZCONNECT\rCRC:BD70\r\r",
        );
        assert!(header.is_err());
    }
}
