use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use bstr::BString;
use chrono::NaiveDateTime;
use jamjam::{
    jam::{
        JamMessage, JamMessageBase, attributes,
        msg_header::{MessageSubfield, SubfieldType},
    },
    util::echmoail::EchomailAddress,
};
use serde::{Deserialize, Serialize};

use super::{
    FtnAka, FtnConfig, bundle,
    packet::{self, PackedMessage, Packet, PacketHeader},
};

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The crc a jam header carries when the message never had a message id.
const NO_MSGID: u32 = 0xffff_ffff;

/// What every message this board sends out says it was written with.
fn product() -> String {
    format!("IcyBoard/{}", env!("CARGO_PKG_VERSION"))
}

/// One area of the board as the tosser sees it: the tag it carries in the
/// network and the message base it is stored in.
pub type AreaMap = [(String, PathBuf)];

/// What one run over the inbound left behind.
#[derive(Debug, Default)]
pub struct TossReport {
    pub imported: usize,
    pub duplicates: usize,
    pub netmail: usize,

    /// Tags that arrived for areas this board does not carry, and how many
    /// messages came with each of them.
    pub unknown: BTreeMap<String, usize>,

    pub failed: Vec<(PathBuf, String)>,
}

/// Reads everything waiting in the inbound into the message bases.
pub fn toss_inbound(config: &FtnConfig, areas: &AreaMap) -> Res<TossReport> {
    let mut report = TossReport::default();
    if !config.inbound.is_dir() {
        return Ok(report);
    }
    let lookup: HashMap<String, PathBuf> = areas.iter().map(|(tag, path)| (tag.to_uppercase(), path.clone())).collect();
    let mut bases = OpenBases::default();

    let mut files = Vec::new();
    for entry in fs::read_dir(&config.inbound)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();

    for file in files {
        let Some(name) = file.file_name().and_then(|name| name.to_str()).map(|name| name.to_ascii_lowercase()) else {
            continue;
        };
        let result = if bundle::is_bundle(&name) {
            toss_bundle(&file, config, &lookup, &mut bases, &mut report)
        } else if bundle::is_packet(&name) {
            toss_packet(&file, config, &lookup, &mut bases, &mut report)
        } else {
            continue;
        };
        match result {
            Ok(()) => fs::remove_file(&file)?,
            // What could not be read stays where it is. Throwing away mail
            // nobody has seen is worse than asking the sysop to look at it.
            Err(err) => report.failed.push((file, err.to_string())),
        }
    }
    Ok(report)
}

fn toss_bundle(file: &Path, config: &FtnConfig, lookup: &HashMap<String, PathBuf>, bases: &mut OpenBases, report: &mut TossReport) -> Res<()> {
    let work = tempfile::tempdir_in(&config.inbound)?;
    for packet in bundle::unpack(file, work.path())? {
        let Some(name) = packet.file_name().and_then(|name| name.to_str()).map(|name| name.to_ascii_lowercase()) else {
            continue;
        };
        if !bundle::is_packet(&name) {
            log::warn!("{} carries {}, which is not mail", file.display(), name);
            continue;
        }
        toss_packet(&packet, config, lookup, bases, report)?;
    }
    Ok(())
}

fn toss_packet(file: &Path, config: &FtnConfig, lookup: &HashMap<String, PathBuf>, bases: &mut OpenBases, report: &mut TossReport) -> Res<()> {
    let packet = Packet::load(file)?;
    for message in &packet.messages {
        match message.area() {
            Some(tag) => {
                let tag = tag.to_string();
                import(message, lookup.get(&tag.to_uppercase()), true, bases, report, &tag)?;
            }
            None => {
                import(message, Some(&config.netmail), false, bases, report, "")?;
                report.netmail += 1;
            }
        }
    }
    Ok(())
}

fn import(message: &PackedMessage, base: Option<&PathBuf>, echo: bool, bases: &mut OpenBases, report: &mut TossReport, tag: &str) -> Res<()> {
    let Some(path) = base.cloned() else {
        *report.unknown.entry(tag.to_uppercase()).or_default() += 1;
        return Ok(());
    };
    let kludges = Kludges::split(&message.text);
    let base = bases.get(&path)?;
    if let Some(id) = &kludges.msgid {
        let crc = JamMessageBase::get_crc(&BString::from(id.as_str()));
        if !base.seen.insert(crc) {
            report.duplicates += 1;
            return Ok(());
        }
    }
    base.base.write_message(&to_jam(message, &kludges, echo))?;
    base.base.write_jhr_header()?;
    if echo {
        report.imported += 1;
    }
    Ok(())
}

fn to_jam(message: &PackedMessage, kludges: &Kludges, echo: bool) -> JamMessage {
    let mut flags = if echo { attributes::MSG_TYPEECHO } else { attributes::MSG_TYPENET };
    for (packed, jam) in [
        (packet::attribute::PRIVATE, attributes::MSG_PRIVATE),
        (packet::attribute::CRASH, attributes::MSG_CRASH),
        (packet::attribute::FILE_ATTACHED, attributes::MSG_FILEATTACH),
        (packet::attribute::FILE_REQUEST, attributes::MSG_FILEREQUEST),
        (packet::attribute::RETURN_RECEIPT_REQUEST, attributes::MSG_RECEIPTREQ),
    ] {
        if message.attributes & packed != 0 {
            flags |= jam;
        }
    }

    let mut jam = JamMessage::default()
        .with_from(BString::from(message.from.as_str()))
        .with_to(BString::from(message.to.as_str()))
        .with_subject(BString::from(message.subject.as_str()))
        .with_date_time(message.written.and_utc())
        .with_attributes(flags)
        .with_text(BString::from(kludges.body.as_str()))
        .with_sub_field(MessageSubfield::new(SubfieldType::Address0, BString::from(message.orig.to_string())))
        .with_sub_field(MessageSubfield::new(SubfieldType::AddressD, BString::from(message.dest.to_string())));

    if let Some(id) = &kludges.msgid {
        jam = jam.with_msg_id(BString::from(id.as_str()));
    }
    if let Some(id) = &kludges.reply {
        jam = jam.with_reply_id(BString::from(id.as_str()));
    }
    if let Some(pid) = &kludges.pid {
        jam = jam.with_sub_field(MessageSubfield::new(SubfieldType::PID, BString::from(pid.as_str())));
    }
    for line in &kludges.seen_by {
        jam = jam.with_sub_field(MessageSubfield::new(SubfieldType::SeenBy2D, BString::from(line.as_str())));
    }
    for line in &kludges.path {
        jam = jam.with_sub_field(MessageSubfield::new(SubfieldType::Path2D, BString::from(line.as_str())));
    }
    for line in &kludges.other {
        jam = jam.with_sub_field(MessageSubfield::new(SubfieldType::FTSKludge, BString::from(line.as_str())));
    }
    jam
}

/// Opening a base means reading every message id it already holds, so a run
/// that meets the same area in packet after packet pays for it once.
#[derive(Default)]
struct OpenBases {
    bases: HashMap<PathBuf, OpenBase>,
}

struct OpenBase {
    base: JamMessageBase,
    seen: HashSet<u32>,
}

impl OpenBases {
    fn get(&mut self, path: &Path) -> Res<&mut OpenBase> {
        if !self.bases.contains_key(path) {
            let base = open_base(path)?;
            let mut seen = HashSet::new();
            for header in base.iter().flatten() {
                if header.msgid_crc != NO_MSGID {
                    seen.insert(header.msgid_crc);
                }
            }
            self.bases.insert(path.to_path_buf(), OpenBase { base, seen });
        }
        Ok(self.bases.get_mut(path).unwrap())
    }
}

fn open_base(path: &Path) -> Res<JamMessageBase> {
    if path.with_extension("jhr").exists() {
        return Ok(JamMessageBase::open(path)?);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(JamMessageBase::create(path.to_path_buf())?)
}

/// The lines a message carries for the machines rather than for the reader.
#[derive(Debug, Default, PartialEq)]
struct Kludges {
    msgid: Option<String>,
    reply: Option<String>,
    pid: Option<String>,
    seen_by: Vec<String>,
    path: Vec<String>,
    other: Vec<String>,
    body: String,
}

impl Kludges {
    /// Jam keeps the kludge lines in fields of their own, and a reader that
    /// showed them to a user would only be showing noise.
    fn split(text: &str) -> Self {
        let mut result = Kludges::default();
        let text = text.replace("\r\n", "\r");
        let text = text.strip_suffix(['\r', '\n']).unwrap_or(&text);
        let mut body: Vec<&str> = Vec::new();

        for (index, line) in text.split(['\r', '\n']).enumerate() {
            if let Some(rest) = line.strip_prefix('\x01') {
                let (name, value) = split_kludge(rest);
                match name.as_str() {
                    "MSGID" => result.msgid = Some(value.to_string()),
                    "REPLY" => result.reply = Some(value.to_string()),
                    "PID" => result.pid = Some(value.to_string()),
                    "PATH" => result.path.push(value.to_string()),
                    _ => result.other.push(rest.to_string()),
                }
            } else if let Some(rest) = line.strip_prefix("SEEN-BY:") {
                result.seen_by.push(rest.trim().to_string());
            } else if index == 0 && line.starts_with("AREA:") {
                // The area said where the message goes, and it has gone there.
            } else {
                body.push(line);
            }
        }
        result.body = body.join("\n");
        result
    }
}

/// A kludge is a name and the rest of the line, and not everybody writes the
/// colon that is supposed to end the name.
fn split_kludge(line: &str) -> (String, &str) {
    let end = line.find([':', ' ']).unwrap_or(line.len());
    let (name, rest) = line.split_at(end);
    (name.to_uppercase(), rest.trim_start_matches([':', ' ']).trim_end())
}

/// What the last run already sent out, kept beside the mail it belongs to.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ScanState {
    /// Counts up over the life of the board so that no two messages written
    /// here are ever offered under the same id.
    #[serde(default)]
    serial: u32,

    #[serde(default)]
    exported: BTreeMap<String, u32>,
}

impl ScanState {
    fn path(config: &FtnConfig) -> PathBuf {
        config.outbound.join("scan.toml")
    }

    fn load(config: &FtnConfig) -> Res<Self> {
        let path = Self::path(config);
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    fn save(&self, config: &FtnConfig) -> Res<()> {
        fs::create_dir_all(&config.outbound)?;
        fs::write(Self::path(config), toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// What one run over the message bases put into the outbound.
#[derive(Debug, Default)]
pub struct ScanReport {
    pub exported: usize,
    pub bundles: Vec<PathBuf>,
}

/// Packs everything written here since the last run into bundles for the links
/// that asked for the area it was written in.
pub fn scan_outbound(config: &FtnConfig, areas: &AreaMap, now: &NaiveDateTime) -> Res<ScanReport> {
    let mut report = ScanReport::default();
    if config.links.is_empty() {
        return Ok(report);
    }
    let mut state = ScanState::load(config)?;
    let mut waiting: Vec<Vec<PackedMessage>> = vec![Vec::new(); config.links.len()];

    for (tag, path) in areas {
        if tag.is_empty() {
            continue;
        }
        let subscribers: Vec<usize> = config
            .links
            .iter()
            .enumerate()
            .filter(|(_, link)| link.carries(tag))
            .map(|(index, _)| index)
            .collect();
        if subscribers.is_empty() || !path.with_extension("jhr").exists() {
            continue;
        }
        let base = JamMessageBase::open(path)?;
        let high = base.active_messages();
        let Some(last) = state.exported.get(tag).copied() else {
            // Nothing was ever sent out of this area, and handing a link the
            // whole history of a board it just met would be rude.
            state.exported.insert(tag.clone(), high);
            continue;
        };

        // The area is fed by whatever address answers for the first link that
        // carries it, because one message can only have one origin.
        let Some(aka) = config.aka_for(&config.links[subscribers[0]]).cloned() else {
            continue;
        };

        for number in (last + 1)..=high {
            let Ok(mut header) = base.read_header(number) else {
                continue;
            };
            // A message that came in from the network carries the echo flag and
            // one written here does not, which is what stops mail going back
            // the way it came.
            if header.attributes & attributes::MSG_TYPEECHO != 0 {
                continue;
            }
            let msgid = match subfield(&header, SubfieldType::MsgID) {
                Some(id) => id,
                None => {
                    state.serial = state.serial.wrapping_add(1);
                    let id = format!("{} {:08x}", aka.address, state.serial);
                    header.msgid_crc = JamMessageBase::get_crc(&BString::from(id.as_str()));
                    header.sub_fields.push(MessageSubfield::new(SubfieldType::MsgID, BString::from(id.as_str())));
                    // The id is written back so that a reply arriving for it
                    // still finds the message it belongs to.
                    base.update_header(number, &header)?;
                    id
                }
            };
            let text = base.read_msg_text(&header)?;
            let message = PackedMessage {
                orig: aka.address.clone(),
                dest: EchomailAddress::default(),
                attributes: 0,
                cost: 0,
                written: chrono::DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_default().naive_utc(),
                to: header.get_to().map(|to| to.to_string()).unwrap_or_default(),
                from: header.get_from().map(|from| from.to_string()).unwrap_or_default(),
                subject: header.get_subject().map(|subject| subject.to_string()).unwrap_or_default(),
                text: exported_text(
                    tag,
                    &msgid,
                    subfield(&header, SubfieldType::ReplyID).as_deref(),
                    &text.to_string(),
                    config,
                    &aka,
                    &subscribers.iter().map(|index| config.links[*index].address.clone()).collect::<Vec<_>>(),
                ),
            };
            for index in &subscribers {
                waiting[*index].push(message.clone());
            }
            report.exported += 1;
        }
        state.exported.insert(tag.clone(), high);
    }

    for (index, messages) in waiting.into_iter().enumerate() {
        if messages.is_empty() {
            continue;
        }
        let link = &config.links[index];
        let Some(aka) = config.aka_for(link) else {
            continue;
        };
        let directory = config.outbound_for(link);
        fs::create_dir_all(&directory)?;

        let mut packet = Packet::new(PacketHeader::new(aka.address.clone(), link.address.clone(), *now, &link.password));
        packet.messages = messages;
        for message in &mut packet.messages {
            message.orig = aka.address.clone();
            message.dest = link.address.clone();
        }

        let work = tempfile::tempdir_in(&directory)?;
        let written = work.path().join(bundle::packet_name(now));
        packet.save(&written)?;
        let name = bundle::next_bundle(&directory, &aka.address, &link.address, now)?;
        bundle::pack(&[written], &name)?;
        report.bundles.push(name);
    }

    state.save(config)?;
    Ok(report)
}

fn subfield(header: &jamjam::jam::msg_header::JamMessageHeader, kind: SubfieldType) -> Option<String> {
    header
        .sub_fields
        .iter()
        .find(|field| *field.get_type() == kind)
        .map(|field| field.get_string().to_string())
}

/// Builds what the other side gets to see: the area, the kludges that say where
/// the message came from, the message itself, and the trail it has travelled.
fn exported_text(tag: &str, msgid: &str, reply: Option<&str>, body: &str, config: &FtnConfig, aka: &FtnAka, links: &[EchomailAddress]) -> String {
    let mut text = format!("AREA:{}\r\x01MSGID: {}\r", tag.to_uppercase(), msgid);
    if let Some(reply) = reply {
        text.push_str(&format!("\x01REPLY: {}\r", reply));
    }
    text.push_str(&format!("\x01PID: {}\r", product()));
    text.push_str(&body.replace("\r\n", "\n").replace('\n', "\r"));
    if !text.ends_with('\r') {
        text.push('\r');
    }
    if !body.contains(" * Origin:") {
        text.push_str(&format!("\r--- {}\r * Origin: {} ({})\r", product(), config.origin, aka.address));
    }

    let mut seen: Vec<(u16, u16)> = links.iter().map(|link| (link.net, link.node)).collect();
    seen.push((aka.address.net, aka.address.node));
    seen.sort_unstable();
    seen.dedup();
    for line in fold(&seen.iter().map(|(net, node)| format!("{}/{}", net, node)).collect::<Vec<_>>()) {
        text.push_str(&format!("SEEN-BY: {}\r", line));
    }
    text.push_str(&format!("\x01PATH: {}/{}\r", aka.address.net, aka.address.node));
    text
}

/// Fts-0004 keeps these lines short enough to read on a terminal.
fn fold(entries: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for entry in entries {
        if !line.is_empty() && line.len() + entry.len() + 1 > 70 {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(entry);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::ftn::{FtnAka, FtnLink};
    use chrono::NaiveDate;

    fn address(text: &str) -> EchomailAddress {
        EchomailAddress::parse(text).unwrap()
    }

    fn when() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2025, 3, 4).unwrap().and_hms_opt(12, 0, 0).unwrap()
    }

    fn config(directory: &Path) -> FtnConfig {
        FtnConfig {
            inbound: directory.join("inbound"),
            outbound: directory.join("outbound"),
            netmail: directory.join("netmail"),
            origin: "A board".to_string(),
            akas: vec![FtnAka {
                address: address("21:1/100"),
                domain: "fsxnet".to_string(),
            }],
            links: vec![FtnLink {
                address: address("21:1/1"),
                domain: "fsxnet".to_string(),
                host: "hub.example".to_string(),
                areas: vec!["FSX_GEN".to_string()],
                ..Default::default()
            }],
        }
    }

    fn message(area: &str, text: &str) -> PackedMessage {
        PackedMessage {
            orig: address("21:1/1"),
            dest: address("21:1/100"),
            written: when(),
            to: "All".to_string(),
            from: "Someone".to_string(),
            subject: "Hello".to_string(),
            text: format!("AREA:{}\r{}", area, text),
            ..Default::default()
        }
    }

    fn drop_packet(config: &FtnConfig, messages: Vec<PackedMessage>) {
        let mut packet = Packet::new(PacketHeader::new(address("21:1/1"), address("21:1/100"), when(), ""));
        packet.messages = messages;
        fs::create_dir_all(&config.inbound).unwrap();
        packet.save(&config.inbound.join(bundle::packet_name(&when()))).unwrap();
    }

    #[test]
    fn test_a_message_from_a_packet_lands_in_the_area_that_carries_its_tag() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![("FSX_GEN".to_string(), directory.path().join("bases/general"))];
        drop_packet(&config, vec![message("FSX_GEN", "\x01MSGID: 21:1/2 11223344\rBody\r")]);

        let report = toss_inbound(&config, &areas).unwrap();

        assert_eq!(report.imported, 1);
        assert!(report.failed.is_empty());
        let base = JamMessageBase::open(&areas[0].1).unwrap();
        let header = base.read_header(1).unwrap();
        assert_eq!(base.read_msg_text(&header).unwrap().to_string(), "Body");
        assert_eq!(header.get_from().unwrap().to_string(), "Someone");
        assert_eq!(subfield(&header, SubfieldType::MsgID).unwrap(), "21:1/2 11223344");
        assert!(fs::read_dir(&config.inbound).unwrap().next().is_none());
    }

    #[test]
    fn test_the_same_message_id_is_only_imported_once() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![("FSX_GEN".to_string(), directory.path().join("bases/general"))];
        let twice = message("FSX_GEN", "\x01MSGID: 21:1/2 11223344\rBody\r");
        drop_packet(&config, vec![twice.clone(), twice]);

        let report = toss_inbound(&config, &areas).unwrap();

        assert_eq!(report.imported, 1);
        assert_eq!(report.duplicates, 1);
    }

    #[test]
    fn test_a_tag_nobody_carries_is_reported_rather_than_dropped_in_silence() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        drop_packet(&config, vec![message("FSX_BBS", "Body\r")]);

        let report = toss_inbound(&config, &[]).unwrap();

        assert_eq!(report.imported, 0);
        assert_eq!(report.unknown.get("FSX_BBS"), Some(&1));
    }

    #[test]
    fn test_netmail_goes_to_the_base_of_its_own() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Hello sysop\r".to_string();
        drop_packet(&config, vec![netmail]);

        let report = toss_inbound(&config, &[]).unwrap();

        assert_eq!(report.netmail, 1);
        let base = JamMessageBase::open(&config.netmail).unwrap();
        assert_eq!(base.active_messages(), 1);
    }

    #[test]
    fn test_a_packet_that_cannot_be_read_is_kept_for_the_sysop() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        fs::create_dir_all(&config.inbound).unwrap();
        let broken = config.inbound.join("12345678.pkt");
        fs::write(&broken, b"not a packet at all").unwrap();

        let report = toss_inbound(&config, &[]).unwrap();

        assert_eq!(report.failed.len(), 1);
        assert!(broken.exists());
    }

    #[test]
    fn test_kludge_lines_are_taken_out_of_the_text() {
        let kludges = Kludges::split("AREA:FSX_GEN\r\x01MSGID: 21:1/2 aabbccdd\r\x01REPLY: 21:1/3 1\rHello\rthere\rSEEN-BY: 1/1 1/100\r\x01PATH: 1/1\r");

        assert_eq!(kludges.msgid.as_deref(), Some("21:1/2 aabbccdd"));
        assert_eq!(kludges.reply.as_deref(), Some("21:1/3 1"));
        assert_eq!(kludges.seen_by, vec!["1/1 1/100".to_string()]);
        assert_eq!(kludges.path, vec!["1/1".to_string()]);
        assert_eq!(kludges.body, "Hello\nthere");
    }

    #[test]
    fn test_the_first_run_of_an_area_sends_nothing_but_remembers_where_it_started() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let path = directory.path().join("bases/general");
        let mut base = open_base(&path).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("old"))).unwrap();
        base.write_jhr_header().unwrap();
        let areas = vec![("FSX_GEN".to_string(), path.clone())];

        let report = scan_outbound(&config, &areas, &when()).unwrap();
        assert_eq!(report.exported, 0);

        base.write_message(&JamMessage::default().with_text(BString::from("new"))).unwrap();
        base.write_jhr_header().unwrap();
        let report = scan_outbound(&config, &areas, &when()).unwrap();
        assert_eq!(report.exported, 1);
    }

    #[test]
    fn test_a_message_written_here_leaves_as_a_bundle_for_the_link_that_asked_for_it() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let path = directory.path().join("bases/general");
        let mut base = open_base(&path).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("first"))).unwrap();
        base.write_jhr_header().unwrap();
        let areas = vec![("FSX_GEN".to_string(), path.clone())];
        scan_outbound(&config, &areas, &when()).unwrap();

        base.write_message(
            &JamMessage::default()
                .with_from(BString::from("Sysop"))
                .with_to(BString::from("All"))
                .with_subject(BString::from("Hi"))
                .with_text(BString::from("Body")),
        )
        .unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &areas, &when()).unwrap();

        assert_eq!(report.exported, 1);
        assert_eq!(report.bundles.len(), 1);
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let packet = Packet::load(&packets[0]).unwrap();
        assert_eq!(packet.header.dest, address("21:1/1"));
        let text = &packet.messages[0].text;
        assert!(text.starts_with("AREA:FSX_GEN\r"), "{:?}", text);
        assert!(text.contains("\x01MSGID: 21:1/100 "), "{:?}", text);
        assert!(text.contains(" * Origin: A board (21:1/100)\r"), "{:?}", text);
        assert!(text.contains("SEEN-BY: 1/1 1/100\r"), "{:?}", text);
        assert!(text.contains("\x01PATH: 1/100\r"), "{:?}", text);
    }

    #[test]
    fn test_what_came_in_from_the_network_is_not_sent_back_out() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let path = directory.path().join("bases/general");
        let areas = vec![("FSX_GEN".to_string(), path.clone())];
        {
            let mut base = open_base(&path).unwrap();
            base.write_message(&JamMessage::default().with_text(BString::from("first"))).unwrap();
            base.write_jhr_header().unwrap();
        }
        scan_outbound(&config, &areas, &when()).unwrap();

        drop_packet(&config, vec![message("FSX_GEN", "\x01MSGID: 21:1/2 11223344\rBody\r")]);
        assert_eq!(toss_inbound(&config, &areas).unwrap().imported, 1);

        let report = scan_outbound(&config, &areas, &when()).unwrap();
        assert_eq!(report.exported, 0);
        assert!(report.bundles.is_empty());
    }
}
