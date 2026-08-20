use std::{
    fs,
    io::{Cursor, Read, Write},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use jamjam::util::echomail::EchomailAddress;
use std::fmt::Write as _;
use thiserror::Error;

/// A packet is a stream of messages, terminated where the next message type
/// would be.
pub const MESSAGE_TYPE: u16 = 2;

/// FSC-0039 marks a type 2+ packet by setting the low bit of the capability
/// word and repeating it byte swapped, so that a reader of plain FTS-0001
/// packets sees a value it will not mistake for anything of its own.
pub const CAPABILITY_TYPE_2_PLUS: u16 = 0x0001;

/// The FTSC has not assigned a code to this program yet.
pub const PRODUCT_CODE: u16 = 0x00fe;

/// The attribute word of FTS-0001, of which a tosser only really acts on a few.
pub mod attribute {
    pub const PRIVATE: u16 = 0x0001;
    pub const CRASH: u16 = 0x0002;
    pub const RECEIVED: u16 = 0x0004;
    pub const SENT: u16 = 0x0008;
    pub const FILE_ATTACHED: u16 = 0x0010;
    pub const IN_TRANSIT: u16 = 0x0020;
    pub const ORPHAN: u16 = 0x0040;
    pub const KILL_SENT: u16 = 0x0080;
    pub const LOCAL: u16 = 0x0100;
    pub const HOLD_FOR_PICKUP: u16 = 0x0200;
    pub const FILE_REQUEST: u16 = 0x0800;
    pub const RETURN_RECEIPT_REQUEST: u16 = 0x1000;
    pub const IS_RETURN_RECEIPT: u16 = 0x2000;
    pub const AUDIT_REQUEST: u16 = 0x4000;
    pub const FILE_UPDATE_REQUEST: u16 = 0x8000;
}

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Packet ends in the middle of a {0}")]
    Truncated(&'static str),

    #[error("Not a fidonet packet: type field says {0}, not 2")]
    NotAPacket(u16),

    #[error("Message {0} in the packet is of type {1}, not 2")]
    UnknownMessageType(usize, u16),
}

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug, PartialEq)]
pub struct PacketHeader {
    pub orig: EchomailAddress,
    pub dest: EchomailAddress,
    pub created: NaiveDateTime,

    /// What the destination expects to see, in clear, up to eight characters.
    pub password: String,

    pub product_code: u16,
    pub revision: (u8, u8),

    /// Kept as read so that a packet passed on unchanged stays what it was.
    pub capability: u16,
}

impl PacketHeader {
    pub const SIZE: usize = 58;

    pub fn new(orig: EchomailAddress, dest: EchomailAddress, created: NaiveDateTime, password: &str) -> Self {
        Self {
            orig,
            dest,
            created,
            password: password.to_string(),
            product_code: PRODUCT_CODE,
            revision: (0, 0),
            capability: CAPABILITY_TYPE_2_PLUS,
        }
    }

    pub fn read(input: &mut impl Read) -> Res<Self> {
        let mut bytes = [0u8; Self::SIZE];
        input.read_exact(&mut bytes).map_err(|_| PacketError::Truncated("packet header"))?;
        let mut cursor = Cursor::new(&bytes[..]);

        let orig_node = cursor.read_u16::<LittleEndian>()?;
        let dest_node = cursor.read_u16::<LittleEndian>()?;
        let year = cursor.read_u16::<LittleEndian>()?;
        let month = cursor.read_u16::<LittleEndian>()?;
        let day = cursor.read_u16::<LittleEndian>()?;
        let hour = cursor.read_u16::<LittleEndian>()?;
        let minute = cursor.read_u16::<LittleEndian>()?;
        let second = cursor.read_u16::<LittleEndian>()?;
        let _baud = cursor.read_u16::<LittleEndian>()?;

        let packet_type = cursor.read_u16::<LittleEndian>()?;
        if packet_type != MESSAGE_TYPE {
            return Err(PacketError::NotAPacket(packet_type).into());
        }

        let orig_net = cursor.read_u16::<LittleEndian>()?;
        let dest_net = cursor.read_u16::<LittleEndian>()?;
        let product_low = cursor.read_u8()?;
        let revision_major = cursor.read_u8()?;

        let mut password = [0u8; 8];
        cursor.read_exact(&mut password)?;

        let orig_zone = cursor.read_u16::<LittleEndian>()?;
        let dest_zone = cursor.read_u16::<LittleEndian>()?;
        let _aux_net = cursor.read_u16::<LittleEndian>()?;
        let _capability_copy = cursor.read_u16::<LittleEndian>()?;
        let product_high = cursor.read_u8()?;
        let revision_minor = cursor.read_u8()?;
        let capability = cursor.read_u16::<LittleEndian>()?;
        let orig_zone_2 = cursor.read_u16::<LittleEndian>()?;
        let dest_zone_2 = cursor.read_u16::<LittleEndian>()?;
        let orig_point = cursor.read_u16::<LittleEndian>()?;
        let dest_point = cursor.read_u16::<LittleEndian>()?;

        // A plain FTS-0001 writer leaves the second pair of zones at zero, and
        // some leave the first pair there as well.
        let orig_zone = if orig_zone_2 != 0 { orig_zone_2 } else { orig_zone };
        let dest_zone = if dest_zone_2 != 0 { dest_zone_2 } else { dest_zone };
        let type_2_plus = capability & CAPABILITY_TYPE_2_PLUS != 0;

        Ok(Self {
            orig: EchomailAddress::new(orig_zone, orig_net, orig_node, if type_2_plus { orig_point } else { 0 }),
            dest: EchomailAddress::new(dest_zone, dest_net, dest_node, if type_2_plus { dest_point } else { 0 }),
            created: to_date_time(year, month, day, hour, minute, second),
            password: read_string(&password),
            product_code: u16::from(product_high) << 8 | u16::from(product_low),
            revision: (revision_major, revision_minor),
            capability,
        })
    }

    pub fn write(&self, output: &mut impl Write) -> Res<()> {
        output.write_u16::<LittleEndian>(self.orig.node)?;
        output.write_u16::<LittleEndian>(self.dest.node)?;
        output.write_u16::<LittleEndian>(self.created.year() as u16)?;
        // FTS-0001 counts the months from zero.
        output.write_u16::<LittleEndian>(self.created.month0() as u16)?;
        output.write_u16::<LittleEndian>(self.created.day() as u16)?;
        output.write_u16::<LittleEndian>(self.created.hour() as u16)?;
        output.write_u16::<LittleEndian>(self.created.minute() as u16)?;
        output.write_u16::<LittleEndian>(self.created.second() as u16)?;
        output.write_u16::<LittleEndian>(0)?;
        output.write_u16::<LittleEndian>(MESSAGE_TYPE)?;
        output.write_u16::<LittleEndian>(self.orig.net)?;
        output.write_u16::<LittleEndian>(self.dest.net)?;
        output.write_u8(self.product_code as u8)?;
        output.write_u8(self.revision.0)?;

        let mut password = [0u8; 8];
        for (target, source) in password.iter_mut().zip(to_cp437(&self.password)) {
            *target = source;
        }
        output.write_all(&password)?;

        output.write_u16::<LittleEndian>(self.orig.zone)?;
        output.write_u16::<LittleEndian>(self.dest.zone)?;
        output.write_u16::<LittleEndian>(0)?;
        output.write_u16::<LittleEndian>(self.capability.swap_bytes())?;
        output.write_u8((self.product_code >> 8) as u8)?;
        output.write_u8(self.revision.1)?;
        output.write_u16::<LittleEndian>(self.capability)?;
        output.write_u16::<LittleEndian>(self.orig.zone)?;
        output.write_u16::<LittleEndian>(self.dest.zone)?;
        output.write_u16::<LittleEndian>(self.orig.point)?;
        output.write_u16::<LittleEndian>(self.dest.point)?;
        output.write_u32::<LittleEndian>(0)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackedMessage {
    pub orig: EchomailAddress,
    pub dest: EchomailAddress,
    pub attributes: u16,
    pub cost: u16,
    pub written: NaiveDateTime,
    pub to: String,
    pub from: String,
    pub subject: String,

    /// Carried as it stands, kludge lines and all, with a carriage return
    /// ending every line the way the rest of fidonet writes them.
    pub text: String,
}

impl PackedMessage {
    /// Echomail is told apart from netmail by the area it names in its first line.
    pub fn area(&self) -> Option<&str> {
        self.text.strip_prefix("AREA:").map(|rest| rest.split(['\r', '\n']).next().unwrap_or("").trim())
    }

    pub fn is_echomail(&self) -> bool {
        self.area().is_some()
    }

    fn read(input: &mut impl Read, zone_of: (u16, u16)) -> Res<Option<Self>> {
        let mut message_type = [0u8; 2];
        match input.read(&mut message_type)? {
            0 => return Ok(None),
            1 => return Err(PacketError::Truncated("message header").into()),
            _ => {}
        }
        let message_type = u16::from_le_bytes(message_type);
        if message_type == 0 {
            return Ok(None);
        }
        if message_type != MESSAGE_TYPE {
            return Err(PacketError::UnknownMessageType(0, message_type).into());
        }

        let orig_node = input.read_u16::<LittleEndian>()?;
        let dest_node = input.read_u16::<LittleEndian>()?;
        let orig_net = input.read_u16::<LittleEndian>()?;
        let dest_net = input.read_u16::<LittleEndian>()?;
        let attributes = input.read_u16::<LittleEndian>()?;
        let cost = input.read_u16::<LittleEndian>()?;

        let mut stamp = [0u8; 20];
        input.read_exact(&mut stamp).map_err(|_| PacketError::Truncated("message header"))?;

        let mut message = Self {
            orig: EchomailAddress::new(zone_of.0, orig_net, orig_node, 0),
            dest: EchomailAddress::new(zone_of.1, dest_net, dest_node, 0),
            attributes,
            cost,
            written: parse_stamp(&read_string(&stamp)),
            to: read_until_nul(input, 36)?,
            from: read_until_nul(input, 36)?,
            subject: read_until_nul(input, 72)?,
            text: read_until_nul(input, usize::MAX)?,
        };
        message.take_addresses_from_kludges();
        Ok(Some(message))
    }

    fn write(&self, output: &mut impl Write) -> Res<()> {
        output.write_u16::<LittleEndian>(MESSAGE_TYPE)?;
        output.write_u16::<LittleEndian>(self.orig.node)?;
        output.write_u16::<LittleEndian>(self.dest.node)?;
        output.write_u16::<LittleEndian>(self.orig.net)?;
        output.write_u16::<LittleEndian>(self.dest.net)?;
        output.write_u16::<LittleEndian>(self.attributes)?;
        output.write_u16::<LittleEndian>(self.cost)?;

        let mut stamp = [0u8; 20];
        for (target, source) in stamp.iter_mut().zip(to_cp437(&format_stamp(&self.written))) {
            *target = source;
        }
        stamp[19] = 0;
        output.write_all(&stamp)?;

        write_nul_terminated(output, &self.to, 35)?;
        write_nul_terminated(output, &self.from, 35)?;
        write_nul_terminated(output, &self.subject, 71)?;
        write_nul_terminated(output, &self.text_with_kludges(), usize::MAX)?;
        Ok(())
    }

    /// The packed header has no room for a zone or a point, so netmail that
    /// needs them says so in the text instead - FTS-4001 and FSC-0035.
    fn text_with_kludges(&self) -> String {
        if self.is_echomail() {
            return self.text.clone();
        }
        let mut prefix = String::new();
        if !has_kludge(&self.text, "INTL") {
            let _ = write!(prefix, "\x01INTL {} {}\r", to_3d(&self.dest), to_3d(&self.orig));
        }
        if self.orig.point != 0 && !has_kludge(&self.text, "FMPT") {
            let _ = write!(prefix, "\x01FMPT {}\r", self.orig.point);
        }
        if self.dest.point != 0 && !has_kludge(&self.text, "TOPT") {
            let _ = write!(prefix, "\x01TOPT {}\r", self.dest.point);
        }
        prefix + self.text.as_str()
    }

    fn take_addresses_from_kludges(&mut self) {
        for line in self.text.split('\r') {
            let Some(kludge) = line.strip_prefix('\x01') else {
                continue;
            };
            let (name, argument) = kludge.split_once(' ').unwrap_or((kludge, ""));
            match name {
                "INTL" => {
                    let mut addresses = argument.split_whitespace();
                    if let Some(dest) = addresses.next().and_then(EchomailAddress::parse) {
                        self.dest.zone = dest.zone;
                    }
                    if let Some(orig) = addresses.next().and_then(EchomailAddress::parse) {
                        self.orig.zone = orig.zone;
                    }
                }
                "FMPT" => self.orig.point = argument.trim().parse().unwrap_or(0),
                "TOPT" => self.dest.point = argument.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub header: PacketHeader,
    pub messages: Vec<PackedMessage>,
}

impl Packet {
    pub fn new(header: PacketHeader) -> Self {
        Self { header, messages: Vec::new() }
    }

    pub fn read(bytes: &[u8]) -> Res<Self> {
        let mut input = Cursor::new(bytes);
        let header = PacketHeader::read(&mut input)?;
        let zone_of = (header.orig.zone, header.dest.zone);

        let mut messages = Vec::new();
        while let Some(message) = PackedMessage::read(&mut input, zone_of).map_err(|err| number(err, messages.len()))? {
            messages.push(message);
        }
        Ok(Self { header, messages })
    }

    pub fn to_bytes(&self) -> Res<Vec<u8>> {
        let mut bytes = Vec::new();
        self.header.write(&mut bytes)?;
        for message in &self.messages {
            message.write(&mut bytes)?;
        }
        bytes.write_u16::<LittleEndian>(0)?;
        Ok(bytes)
    }

    pub fn load(path: &Path) -> Res<Self> {
        Self::read(&fs::read(path)?)
    }

    pub fn save(&self, path: &Path) -> Res<()> {
        fs::write(path, self.to_bytes()?)?;
        Ok(())
    }
}

/// The message reader does not know its own index, so it is filled in here.
fn number(error: Box<dyn std::error::Error + Send + Sync>, index: usize) -> Box<dyn std::error::Error + Send + Sync> {
    match error.downcast::<PacketError>() {
        Ok(packet_error) => match *packet_error {
            PacketError::UnknownMessageType(_, message_type) => PacketError::UnknownMessageType(index, message_type).into(),
            other => other.into(),
        },
        Err(other) => other,
    }
}

fn to_3d(address: &EchomailAddress) -> String {
    format!("{}:{}/{}", address.zone, address.net, address.node)
}

fn has_kludge(text: &str, name: &str) -> bool {
    text.split('\r').any(|line| line.starts_with(&format!("\x01{name} ")))
}

fn to_date_time(year: u16, month: u16, day: u16, hour: u16, minute: u16, second: u16) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(i32::from(year), u32::from(month.min(11)) + 1, u32::from(day.max(1)))
        .and_then(|date| date.and_hms_opt(u32::from(hour), u32::from(minute), u32::from(second)))
        .unwrap_or_default()
}

/// FTS-0001 writes "01 Jan 86  02:34:56", but the number of spaces in the
/// middle is not something every writer agrees on.
fn parse_stamp(stamp: &str) -> NaiveDateTime {
    let normalized = stamp.split_whitespace().collect::<Vec<_>>().join(" ");
    NaiveDateTime::parse_from_str(&normalized, "%d %b %y %H:%M:%S").unwrap_or_default()
}

fn format_stamp(written: &NaiveDateTime) -> String {
    written.format("%d %b %y  %H:%M:%S").to_string()
}

fn read_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    from_cp437(&bytes[..end])
}

fn read_until_nul(input: &mut impl Read, limit: usize) -> Res<String> {
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if input.read(&mut byte)? == 0 {
            return Err(PacketError::Truncated("message").into());
        }
        if byte[0] == 0 {
            break;
        }
        if bytes.len() < limit {
            bytes.push(byte[0]);
        }
    }
    Ok(from_cp437(&bytes))
}

fn write_nul_terminated(output: &mut impl Write, text: &str, limit: usize) -> Res<()> {
    let mut bytes = to_cp437(text);
    bytes.truncate(limit);
    output.write_all(&bytes)?;
    output.write_u8(0)?;
    Ok(())
}

/// A kludge line is marked with a control character, so the glyphs cp437 gives
/// the low bytes must not be applied to anything that comes out of a packet.
fn from_cp437(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            if *byte < 0x80 {
                *byte as char
            } else {
                codepages::tables::CP437_TO_UNICODE[*byte as usize]
            }
        })
        .collect()
}

fn to_cp437(text: &str) -> Vec<u8> {
    text.chars()
        .map(|ch| {
            if (ch as u32) < 0x80 {
                ch as u8
            } else {
                *codepages::tables::UNICODE_TO_CP437.get(&ch).unwrap_or(&b'.')
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(text: &str) -> EchomailAddress {
        EchomailAddress::parse(text).unwrap()
    }

    fn written() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 9).unwrap().and_hms_opt(14, 30, 5).unwrap()
    }

    fn packet() -> Packet {
        let mut packet = Packet::new(PacketHeader::new(address("21:1/100"), address("21:1/1"), written(), "secret"));
        packet.messages.push(PackedMessage {
            orig: address("21:1/100"),
            dest: address("21:1/1"),
            attributes: attribute::PRIVATE,
            written: written(),
            to: "Sysop".to_string(),
            from: "Mike".to_string(),
            subject: "Hello".to_string(),
            text: "First line\rSecond line\r".to_string(),
            ..Default::default()
        });
        packet
    }

    #[test]
    fn test_a_packet_survives_a_round_trip() {
        let written = packet();
        let read = Packet::read(&written.to_bytes().unwrap()).unwrap();

        assert_eq!(read.header, written.header);
        assert_eq!(read.messages.len(), 1);
        assert_eq!(read.messages[0].from, "Mike");
        assert_eq!(read.messages[0].subject, "Hello");
        assert_eq!(read.messages[0].written, written.messages[0].written);
    }

    #[test]
    fn test_the_header_is_the_fifty_eight_octets_fts_0001_describes() {
        let mut bytes = Vec::new();
        packet().header.write(&mut bytes).unwrap();

        assert_eq!(bytes.len(), PacketHeader::SIZE);
        assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), MESSAGE_TYPE);
        assert_eq!(&bytes[26..32], b"secret");
        assert_eq!(u16::from_le_bytes([bytes[44], bytes[45]]), CAPABILITY_TYPE_2_PLUS);
        assert_eq!(u16::from_le_bytes([bytes[40], bytes[41]]), CAPABILITY_TYPE_2_PLUS.swap_bytes());
    }

    #[test]
    fn test_a_packet_ends_where_the_next_message_type_would_be() {
        let bytes = packet().to_bytes().unwrap();

        assert_eq!(&bytes[bytes.len() - 2..], &[0, 0]);
    }

    #[test]
    fn test_a_point_travels_in_the_text_because_the_header_has_no_room_for_it() {
        let mut packet = packet();
        packet.messages[0].orig = address("21:1/100.7");
        packet.messages[0].dest = address("2:280/1");

        let read = Packet::read(&packet.to_bytes().unwrap()).unwrap();

        assert!(read.messages[0].text.starts_with("\x01INTL 2:280/1 21:1/100\r\x01FMPT 7\r"));
        assert_eq!(read.messages[0].orig, address("21:1/100.7"));
        assert_eq!(read.messages[0].dest, address("2:280/1"));
    }

    #[test]
    fn test_a_kludge_that_is_already_there_is_not_written_twice() {
        let mut packet = packet();
        packet.messages[0].text = "\x01INTL 2:280/1 21:1/100\rBody\r".to_string();
        packet.messages[0].dest = address("2:280/1");

        let read = Packet::read(&packet.to_bytes().unwrap()).unwrap();

        assert_eq!(read.messages[0].text.matches("\x01INTL").count(), 1);
    }

    #[test]
    fn test_echomail_says_which_area_it_belongs_to_and_gets_no_intl() {
        let mut packet = packet();
        packet.messages[0].text = "AREA:FSX_GEN\rBody\r".to_string();

        let read = Packet::read(&packet.to_bytes().unwrap()).unwrap();

        assert_eq!(read.messages[0].area(), Some("FSX_GEN"));
        assert!(!read.messages[0].text.contains("\x01INTL"));
    }

    #[test]
    fn test_a_message_that_is_not_of_type_two_is_refused() {
        let mut bytes = packet().to_bytes().unwrap();
        bytes[PacketHeader::SIZE] = 3;

        assert!(Packet::read(&bytes).is_err());
    }

    #[test]
    fn test_a_packet_that_stops_in_the_middle_is_refused() {
        let bytes = packet().to_bytes().unwrap();

        assert!(Packet::read(&bytes[..PacketHeader::SIZE - 1]).is_err());
        assert!(Packet::read(&bytes[..bytes.len() - 3]).is_err());
    }

    #[test]
    fn test_a_name_longer_than_the_field_is_cut_and_not_the_message_after_it() {
        let mut packet = packet();
        packet.messages[0].to = "N".repeat(80);

        let read = Packet::read(&packet.to_bytes().unwrap()).unwrap();

        assert_eq!(read.messages[0].to.len(), 35);
        assert_eq!(read.messages[0].from, "Mike");
    }

    #[test]
    fn test_a_stamp_is_read_no_matter_how_it_was_spaced() {
        assert_eq!(parse_stamp("09 Aug 26  14:30:05"), written());
        assert_eq!(parse_stamp("09 Aug 26 14:30:05"), written());
    }

    #[test]
    fn test_a_type_two_packet_without_the_capability_word_is_still_read() {
        let mut bytes = packet().to_bytes().unwrap();
        bytes[40..46].fill(0);
        bytes[46..54].fill(0);

        let read = Packet::read(&bytes).unwrap();

        assert_eq!(read.header.orig, address("21:1/100"));
        assert_eq!(read.header.dest, address("21:1/1"));
    }
}
