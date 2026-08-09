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
    FtnAka, FtnConfig, FtnLink, bundle,
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

/// What the tosser needs to know about the board that `ftn.toml` does not say.
#[derive(Debug, Default)]
pub struct TossTarget {
    /// The name netmail addressed to the sysop is delivered under.
    pub sysop: String,

    /// Every name this board would hand netmail to.
    pub users: Vec<String>,
}

/// What one run over the inbound left behind.
#[derive(Debug, Default)]
pub struct TossReport {
    pub imported: usize,
    pub duplicates: usize,
    pub netmail: usize,

    /// Tags that arrived for areas this board does not carry, and how many
    /// messages came with each of them.
    pub unknown: BTreeMap<String, usize>,

    /// Tags that were given an area of their own, and the base each got.
    pub added: BTreeMap<String, PathBuf>,

    /// Messages handed on to a downlink without being stored here.
    pub passed_through: usize,

    /// Packets addressed to another system, left where they were found.
    pub orphans: usize,

    /// The bundles the passed through mail was packed into.
    pub bundles: Vec<PathBuf>,

    pub failed: Vec<(PathBuf, String)>,
}

/// Reads everything waiting in the inbound into the message bases.
pub fn toss_inbound(config: &FtnConfig, areas: &AreaMap, target: &TossTarget) -> Res<TossReport> {
    let mut report = TossReport::default();
    if !config.options.process_in || !config.inbound.is_dir() {
        return Ok(report);
    }
    let mut tosser = Tosser {
        config,
        target,
        lookup: areas.iter().map(|(tag, path)| (tag.to_uppercase(), path.clone())).collect(),
        bases: OpenBases::new(config.options.msgs_to_track),
        forward: vec![Vec::new(); config.links.len()],
    };

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
            tosser.bundle(&file, &mut report)
        } else if bundle::is_packet(&name) {
            tosser.packet(&file, &mut report)
        } else {
            continue;
        };
        match result {
            Ok(true) => fs::remove_file(&file)?,
            // Mail for somebody else stays where it is, and so does what could
            // not be read. Throwing away mail nobody has seen is worse than
            // asking the sysop to look at it.
            Ok(false) => {}
            Err(err) => report.failed.push((file, err.to_string())),
        }
    }
    tosser.send_on(&mut report)?;
    Ok(report)
}

struct Tosser<'a> {
    config: &'a FtnConfig,
    target: &'a TossTarget,
    lookup: HashMap<String, PathBuf>,
    bases: OpenBases,

    /// Mail for a tag no area here carries, waiting for the links that do.
    forward: Vec<Vec<PackedMessage>>,
}

impl Tosser<'_> {
    fn bundle(&mut self, file: &Path, report: &mut TossReport) -> Res<bool> {
        let work = tempfile::tempdir_in(&self.config.inbound)?;
        let mut done = true;
        for packet in bundle::unpack(file, work.path())? {
            let Some(name) = packet.file_name().and_then(|name| name.to_str()).map(|name| name.to_ascii_lowercase()) else {
                continue;
            };
            if !bundle::is_packet(&name) {
                log::warn!("{} carries {}, which is not mail", file.display(), name);
                continue;
            }
            done &= self.packet(&packet, report)?;
        }
        Ok(done)
    }

    fn packet(&mut self, file: &Path, report: &mut TossReport) -> Res<bool> {
        let mut packet = Packet::load(file)?;
        self.complete(&mut packet.header.orig);
        self.complete(&mut packet.header.dest);
        if !self.config.options.process_orphan && !self.config.akas.is_empty() && !self.config.answers_to(&packet.header.dest) {
            report.orphans += 1;
            return Ok(false);
        }
        for message in &packet.messages {
            match message.area() {
                Some(tag) => {
                    let tag = tag.to_string();
                    self.echomail(message, &tag, &packet.header.orig, report)?;
                }
                None => self.netmail(message, report)?,
            }
        }
        Ok(true)
    }

    /// A packet written by a two dimensional system leaves the zone at zero,
    /// and only the sysop knows which network it meant.
    fn complete(&self, address: &mut EchomailAddress) {
        if address.zone == 0 {
            address.zone = self.config.options.default_zone;
        }
        if address.net == 0 {
            address.net = self.config.options.default_net;
        }
    }

    fn echomail(&mut self, message: &PackedMessage, tag: &str, from: &EchomailAddress, report: &mut TossReport) -> Res<()> {
        if let Some(path) = self.lookup.get(&tag.to_uppercase()).cloned() {
            return self.import(message, &path, true, report);
        }
        if self.config.options.auto_add {
            let path = self.config.new_areas.join(tag.to_lowercase());
            self.lookup.insert(tag.to_uppercase(), path.clone());
            report.added.insert(tag.to_uppercase(), path.clone());
            return self.import(message, &path, true, report);
        }
        if self.config.options.pass_thru && self.hand_on(tag, from, message) {
            report.passed_through += 1;
            return Ok(());
        }
        *report.unknown.entry(tag.to_uppercase()).or_default() += 1;
        Ok(())
    }

    fn netmail(&mut self, message: &PackedMessage, report: &mut TossReport) -> Res<()> {
        let mut message = message.clone();
        if self.config.options.sysop_change && !self.target.sysop.is_empty() && message.to.eq_ignore_ascii_case("sysop") {
            message.to = self.target.sysop.clone();
        }
        let known = message.to.eq_ignore_ascii_case(&self.target.sysop) || self.target.users.iter().any(|name| name.eq_ignore_ascii_case(&message.to));
        let base = if self.config.options.secure && !known {
            self.config.bad_netmail.clone()
        } else {
            self.config.netmail.clone()
        };
        self.import(&message, &base, false, report)?;
        report.netmail += 1;
        Ok(())
    }

    fn import(&mut self, message: &PackedMessage, path: &Path, echo: bool, report: &mut TossReport) -> Res<()> {
        let kludges = Kludges::split(&message.text);
        if self.config.options.check_dupe_path && self.travelled_here(&kludges.path) {
            report.duplicates += 1;
            return Ok(());
        }
        let base = self.bases.get(path)?;
        if self.config.options.check_dupe_msg_id {
            if let Some(id) = &kludges.msgid {
                let crc = JamMessageBase::get_crc(&BString::from(id.as_str()));
                if !base.seen.insert(crc) {
                    report.duplicates += 1;
                    return Ok(());
                }
            }
        }
        base.base.write_message(&to_jam(message, &kludges, echo))?;
        base.base.write_jhr_header()?;
        if echo {
            report.imported += 1;
        }
        Ok(())
    }

    /// A message whose path already names this board has been here before.
    fn travelled_here(&self, path: &[String]) -> bool {
        let mine: HashSet<(u16, u16)> = self.config.akas.iter().map(|aka| (aka.address.net, aka.address.node)).collect();
        path.iter().flat_map(|line| nodes(line)).any(|node| mine.contains(&node))
    }

    /// An area this board does not carry is still worth passing on when a link
    /// asked for it, which is what makes a hub out of a node.
    fn hand_on(&mut self, tag: &str, from: &EchomailAddress, message: &PackedMessage) -> bool {
        let seen = seen_by(&message.text);
        let takers: Vec<usize> = self
            .config
            .links
            .iter()
            .enumerate()
            .filter(|(_, link)| {
                link.carries(tag) && (link.address.net, link.address.node) != (from.net, from.node) && !seen.contains(&(link.address.net, link.address.node))
            })
            .map(|(index, _)| index)
            .collect();
        let Some(first) = takers.first() else {
            return false;
        };
        let Some(aka) = self.config.aka_for(&self.config.links[*first]).cloned() else {
            return false;
        };
        let addresses: Vec<EchomailAddress> = takers.iter().map(|index| self.config.links[*index].address.clone()).collect();
        let mut passed = message.clone();
        passed.text = handed_on_text(&message.text, &aka, &addresses);
        for index in takers {
            self.forward[index].push(passed.clone());
        }
        true
    }

    fn send_on(&mut self, report: &mut TossReport) -> Res<()> {
        let now = chrono::Local::now().naive_local();
        for (index, messages) in std::mem::take(&mut self.forward).into_iter().enumerate() {
            if messages.is_empty() {
                continue;
            }
            let link = &self.config.links[index];
            let Some(aka) = self.config.aka_for(link) else {
                continue;
            };
            report.bundles.push(deliver(self.config, link, aka, messages, &now)?);
        }
        Ok(())
    }
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

/// Opening a base means reading the message ids it already holds, so a run
/// that meets the same area in packet after packet pays for it once.
struct OpenBases {
    /// How far back the duplicate check looks, zero for the whole base.
    track: u32,
    bases: HashMap<PathBuf, OpenBase>,
}

struct OpenBase {
    base: JamMessageBase,
    seen: HashSet<u32>,
}

impl OpenBases {
    fn new(track: u32) -> Self {
        Self { track, bases: HashMap::new() }
    }

    fn get(&mut self, path: &Path) -> Res<&mut OpenBase> {
        if !self.bases.contains_key(path) {
            let base = open_base(path)?;
            let mut ids: Vec<u32> = base.iter().flatten().map(|header| header.msgid_crc).filter(|crc| *crc != NO_MSGID).collect();
            if self.track > 0 && ids.len() > self.track as usize {
                ids.drain(..ids.len() - self.track as usize);
            }
            let seen = ids.into_iter().collect();
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
    if !config.options.process_out || config.links.is_empty() {
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
        report.bundles.push(deliver(config, link, aka, messages, now)?);
    }

    state.save(config)?;
    Ok(report)
}

/// Puts what is waiting for one link into a bundle of its own.
fn deliver(config: &FtnConfig, link: &FtnLink, aka: &FtnAka, mut messages: Vec<PackedMessage>, now: &NaiveDateTime) -> Res<PathBuf> {
    let directory = config.outbound_for(link);
    fs::create_dir_all(&directory)?;

    for message in &mut messages {
        message.orig = aka.address.clone();
        message.dest = link.address.clone();
    }
    let mut packet = Packet::new(PacketHeader::new(aka.address.clone(), link.address.clone(), *now, &link.password));
    packet.messages = messages;

    let work = tempfile::tempdir_in(&directory)?;
    let written = work.path().join(bundle::packet_name(now));
    packet.save(&written)?;
    let name = bundle::next_bundle(&directory, &aka.address, &link.address, now)?;
    bundle::pack(&[written], &name)?;
    Ok(name)
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

/// What a message that is only travelling through gets: the systems it is
/// being handed to, so that it is not offered to them a second time.
fn handed_on_text(text: &str, aka: &FtnAka, links: &[EchomailAddress]) -> String {
    let mut out = text.trim_end_matches('\r').to_string();
    out.push('\r');
    let mut seen: Vec<(u16, u16)> = links.iter().map(|link| (link.net, link.node)).collect();
    seen.push((aka.address.net, aka.address.node));
    seen.sort_unstable();
    seen.dedup();
    for line in fold(&seen.iter().map(|(net, node)| format!("{}/{}", net, node)).collect::<Vec<_>>()) {
        out.push_str(&format!("SEEN-BY: {}\r", line));
    }
    out.push_str(&format!("\x01PATH: {}/{}\r", aka.address.net, aka.address.node));
    out
}

/// Every system a message has already been offered to.
fn seen_by(text: &str) -> HashSet<(u16, u16)> {
    text.split(['\r', '\n'])
        .filter_map(|line| line.strip_prefix("SEEN-BY:"))
        .flat_map(nodes)
        .collect()
}

/// A seen-by or path line names a net once and then lists the nodes in it.
fn nodes(line: &str) -> Vec<(u16, u16)> {
    let mut result = Vec::new();
    let mut net = 0;
    for entry in line.split_whitespace() {
        match entry.split_once('/') {
            Some((left, right)) => {
                if let (Ok(parsed), Ok(node)) = (left.parse::<u16>(), right.parse::<u16>()) {
                    net = parsed;
                    result.push((net, node));
                }
            }
            None => {
                if let Ok(node) = entry.parse::<u16>() {
                    result.push((net, node));
                }
            }
        }
    }
    result
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
            bad_netmail: directory.join("badmail"),
            new_areas: directory.join("areas"),
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
            ..Default::default()
        }
    }

    fn toss(config: &FtnConfig, areas: &AreaMap) -> TossReport {
        toss_inbound(config, areas, &TossTarget::default()).unwrap()
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

        let report = toss(&config, &areas);

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

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 1);
        assert_eq!(report.duplicates, 1);
    }

    #[test]
    fn test_the_duplicate_check_can_be_switched_off() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.check_dupe_msg_id = false;
        let areas = vec![("FSX_GEN".to_string(), directory.path().join("bases/general"))];
        let twice = message("FSX_GEN", "\x01MSGID: 21:1/2 11223344\rBody\r");
        drop_packet(&config, vec![twice.clone(), twice]);

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 2);
        assert_eq!(report.duplicates, 0);
    }

    #[test]
    fn test_a_message_that_has_been_here_before_is_recognised_by_its_path() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.check_dupe_path = true;
        let areas = vec![("FSX_GEN".to_string(), directory.path().join("bases/general"))];
        drop_packet(&config, vec![message("FSX_GEN", "Body\r\x01PATH: 1/2 100\r")]);

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 0);
        assert_eq!(report.duplicates, 1);
    }

    #[test]
    fn test_a_packet_for_another_system_is_left_alone_unless_orphans_are_wanted() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![("FSX_GEN".to_string(), directory.path().join("bases/general"))];
        let mut packet = Packet::new(PacketHeader::new(address("21:1/1"), address("21:1/200"), when(), ""));
        packet.messages = vec![message("FSX_GEN", "Body\r")];
        fs::create_dir_all(&config.inbound).unwrap();
        let file = config.inbound.join(bundle::packet_name(&when()));
        packet.save(&file).unwrap();

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 0);
        assert_eq!(report.orphans, 1);
        assert!(file.exists());
    }

    #[test]
    fn test_an_unknown_tag_becomes_an_area_of_its_own_when_auto_add_says_so() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.auto_add = true;
        drop_packet(&config, vec![message("FSX_BBS", "Body\r")]);

        let report = toss(&config, &[]);

        assert_eq!(report.imported, 1);
        assert!(report.unknown.is_empty());
        let path = report.added.get("FSX_BBS").unwrap();
        assert_eq!(JamMessageBase::open(path).unwrap().active_messages(), 1);
    }

    #[test]
    fn test_a_tag_only_a_downlink_carries_is_handed_on_instead_of_stored() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.pass_thru = true;
        config.links[0].areas.push("FSX_BBS".to_string());
        // The message comes from somewhere else, or the link would be sent its
        // own mail back.
        let mut packet = Packet::new(PacketHeader::new(address("21:2/2"), address("21:1/100"), when(), ""));
        packet.messages = vec![message("FSX_BBS", "Body\r")];
        fs::create_dir_all(&config.inbound).unwrap();
        packet.save(&config.inbound.join(bundle::packet_name(&when()))).unwrap();

        let report = toss(&config, &[]);

        assert_eq!(report.imported, 0);
        assert_eq!(report.passed_through, 1);
        assert_eq!(report.bundles.len(), 1);
        assert!(report.bundles[0].exists());
    }

    #[test]
    fn test_netmail_for_a_name_nobody_carries_is_kept_apart_on_a_secure_board() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.secure = true;
        let mut stranger = message("FSX_GEN", "");
        stranger.text = "Who are you\r".to_string();
        stranger.to = "Nobody Here".to_string();
        let mut known = stranger.clone();
        known.to = "Sysop".to_string();
        drop_packet(&config, vec![stranger, known]);

        let target = TossTarget {
            sysop: "The Sysop".to_string(),
            users: Vec::new(),
        };
        let report = toss_inbound(&config, &[], &target).unwrap();

        assert_eq!(report.netmail, 2);
        assert_eq!(JamMessageBase::open(&config.bad_netmail).unwrap().active_messages(), 1);
        let base = JamMessageBase::open(&config.netmail).unwrap();
        assert_eq!(base.read_header(1).unwrap().get_to().unwrap().to_string(), "The Sysop");
    }

    #[test]
    fn test_a_tag_nobody_carries_is_reported_rather_than_dropped_in_silence() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        drop_packet(&config, vec![message("FSX_BBS", "Body\r")]);

        let report = toss(&config, &[]);

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

        let report = toss(&config, &[]);

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

        let report = toss(&config, &[]);

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
        assert_eq!(toss(&config, &areas).imported, 1);

        let report = scan_outbound(&config, &areas, &when()).unwrap();
        assert_eq!(report.exported, 0);
        assert!(report.bundles.is_empty());
    }
}
