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
        raw,
    },
    util::echomail::EchomailAddress,
};
use serde::{Deserialize, Serialize};

use super::{
    Context, FtnAka, FtnConfig, FtnLink, bundle,
    packet::{self, PackedMessage, Packet, PacketHeader},
};
use std::fmt::Write as _;

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The crc a jam header carries when the message never had a message id.
const NO_MSGID: u32 = 0xffff_ffff;

/// What every message this board sends out says it was written with.
fn product() -> String {
    format!("IcyBoard/{}", env!("CARGO_PKG_VERSION"))
}

/// One area of the board as the tosser sees it: the tag it carries in the
/// network and the message base it is stored in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EchoArea {
    pub tag: String,
    pub path: PathBuf,

    /// Takes the place of the board wide origin line, empty when that one applies.
    pub origin: String,
}

impl EchoArea {
    pub fn new(tag: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            tag: tag.into(),
            path: path.into(),
            origin: String::new(),
        }
    }
}

pub type AreaMap = [EchoArea];

/// What one run over the inbound left behind.
#[derive(Debug, Default)]
pub struct TossReport {
    pub imported: usize,
    pub duplicates: usize,
    pub netmail: usize,

    /// Unconfigured source nodes that sent netmail, and how many messages were
    /// kept apart for each one.
    pub untrusted_netmail: BTreeMap<String, usize>,

    /// Tags that arrived for areas this board does not carry, and how many
    /// messages came with each of them.
    pub unknown: BTreeMap<String, usize>,

    /// Tags that were given an area of their own, and the base each got.
    pub added: BTreeMap<String, PathBuf>,

    /// Messages handed on to a downlink without being stored here.
    pub passed_through: usize,
    pub routed: usize,
    pub area_fix: usize,

    /// Packets addressed to another system, left where they were found.
    pub orphans: usize,

    /// The bundles the passed through mail was packed into.
    pub bundles: Vec<PathBuf>,

    pub failed: Vec<(PathBuf, String)>,

    /// Area subscriptions changed by `AreaFix`, keyed by link index.
    pub link_updates: BTreeMap<usize, Vec<String>>,
}

/// Reads everything waiting in the inbound into the message bases.
pub fn toss_inbound(config: &FtnConfig, areas: &AreaMap) -> Res<TossReport> {
    let mut report = TossReport::default();
    if !config.options.enabled || !config.options.process_in || !config.inbound.is_dir() {
        return Ok(report);
    }
    let mut tosser = Tosser {
        config,
        lookup: areas.iter().map(|area| (area.tag.to_uppercase(), area.path.clone())).collect(),
        bases: OpenBases::new(config.options.msgs_to_track),
        forward: vec![Vec::new(); config.links.len()],
        link_areas: config.links.iter().map(|link| link.areas.clone()).collect(),
        routed: Vec::new(),
    };

    let mut files = Vec::new();
    for entry in fs::read_dir(&config.inbound).context(|| format!("Cannot read the inbound {}", config.inbound.display()))? {
        let entry = entry.context(|| format!("Cannot read the inbound {}", config.inbound.display()))?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();

    for file in files {
        let Some(name) = file.file_name().and_then(|name| name.to_str()).map(str::to_ascii_lowercase) else {
            continue;
        };
        log::info!("Tossing {}", file.display());
        let result = if bundle::is_bundle(&name) {
            tosser.bundle(&file, &mut report)
        } else if bundle::is_packet(&name) {
            tosser.packet(&file, &mut report)
        } else {
            continue;
        };
        match result {
            Ok(true) => fs::remove_file(&file).context(|| format!("Cannot remove {} now that it has been tossed", file.display()))?,
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
    lookup: HashMap<String, PathBuf>,
    bases: OpenBases,

    /// Mail for a tag no area here carries, waiting for the links that do.
    forward: Vec<Vec<PackedMessage>>,
    link_areas: Vec<Vec<String>>,

    /// Complete packets routed through a next-hop link.
    routed: Vec<(usize, Packet)>,
}

impl Tosser<'_> {
    fn bundle(&mut self, file: &Path, report: &mut TossReport) -> Res<bool> {
        let work = tempfile::tempdir_in(&self.config.inbound).context(|| {
            format!(
                "Cannot create a working directory in the inbound {} to unpack {}",
                self.config.inbound.display(),
                file.display()
            )
        })?;
        let mut done = true;
        for path in bundle::unpack(file, work.path())? {
            let Some(name) = path.file_name().and_then(|name| name.to_str()).map(str::to_ascii_lowercase) else {
                continue;
            };
            if !bundle::is_packet(&name) {
                log::warn!(
                    "{} carries {}, which is not a packet. The bundle is left in the inbound so the file can be inspected or recovered by hand",
                    file.display(),
                    name
                );
                done = false;
                continue;
            }
            // The unpacked packet lives in a working directory, so its own path
            // would name a file the sysop can no longer look at.
            let bytes = fs::read(&path).context(|| format!("Cannot read {} unpacked from the bundle {}", name, file.display()))?;
            let packet = Packet::read(&bytes).context(|| format!("Cannot read {} out of the bundle {}", name, file.display()))?;
            done &= self.toss_packet(packet, &format!("{} out of {}", name, file.display()), report)?;
        }
        Ok(done)
    }

    fn packet(&mut self, file: &Path, report: &mut TossReport) -> Res<bool> {
        let packet = Packet::load(file)?;
        let name = file.display().to_string();
        self.toss_packet(packet, &name, report)
    }

    /// `where_from` names the mail the way the sysop met it, which is the
    /// bundle for a packet that only ever existed while it was being unpacked.
    fn toss_packet(&mut self, mut packet: Packet, where_from: &str, report: &mut TossReport) -> Res<bool> {
        self.complete(&mut packet.header.orig);
        self.complete(&mut packet.header.dest);
        log::debug!(
            "{} contains {} message(s), from {} to {}",
            where_from,
            packet.messages.len(),
            packet.header.orig,
            packet.header.dest
        );
        if let Some(link) = self.config.links.iter().find(|link| link.address == packet.header.orig)
            && !link.packet_password.is_empty()
            && !packet_password_matches(&link.packet_password, &packet.header.password)
        {
            return Err(format!(
                "{} claims to come from {}, but its packet password is not the one packet_password names for that link. Correct it in ftn.toml, or clear it to take the packets as they come",
                where_from, packet.header.orig
            )
            .into());
        }
        if !self.config.answers_to(&packet.header.dest)
            && self.config.options.enable_routing
            && let Some((_, via)) = self.config.route_for(&packet.header.dest)
        {
            if self.config.links[via].address == packet.header.orig {
                return Err(format!(
                    "{} would route the packet for {} straight back to {}, so it is left in the inbound. Correct the route's next hop",
                    where_from, packet.header.dest, packet.header.orig
                )
                .into());
            }
            if self.config.options.re_address {
                let link = &self.config.links[via];
                let Some(aka) = self.config.aka_for(link) else {
                    return Err(format!("No local address can route {} through {}", packet.header.dest, link.to_5d()).into());
                };
                packet.header.orig = aka.address;
                packet.header.dest = link.address;
                packet.header.password.clone_from(&link.packet_password);
            }
            self.routed.push((via, packet));
            report.routed += 1;
            return Ok(true);
        }
        if !self.config.options.process_orphan && !self.config.akas.is_empty() && !self.config.answers_to(&packet.header.dest) {
            log::warn!(
                "{} is addressed to {} and this board answers to {}, so it is left in the inbound. Add that address as an aka, or turn on Toss Orphans to read mail meant for another system",
                where_from,
                packet.header.dest,
                self.config.akas.iter().map(FtnAka::to_5d).collect::<Vec<_>>().join(", ")
            );
            report.orphans += 1;
            return Ok(false);
        }
        for incoming in &packet.messages {
            let mut message = incoming.clone();
            if self.config.options.sysop_change && message.to.eq_ignore_ascii_case("sysop") {
                message.to = "FIDO_SYSOP".to_string();
            }
            match message.area() {
                Some(tag) => {
                    let tag = tag.to_string();
                    self.echomail(&message, &tag, &packet.header.orig, report)?;
                }
                None => self.netmail(&message, &packet.header.orig, report)?,
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

    fn netmail(&mut self, message: &PackedMessage, source: &EchomailAddress, report: &mut TossReport) -> Res<()> {
        let source_link = self.config.links.iter().position(|link| link.address == *source);
        let trusted = source_link.is_some();
        let untrusted = self.config.options.secure && !trusted;
        let base = if untrusted {
            self.config.bad_netmail.clone()
        } else {
            self.config.netmail.clone()
        };
        let duplicates = report.duplicates;
        self.import(message, &base, false, report)?;
        if message.to.eq_ignore_ascii_case("areafix")
            && report.duplicates == duplicates
            && let Some(index) = source_link
        {
            self.area_fix(index, message, report);
        }
        if untrusted {
            *report.untrusted_netmail.entry(source.to_string()).or_default() += 1;
        }
        report.netmail += 1;
        Ok(())
    }

    fn area_fix(&mut self, index: usize, message: &PackedMessage, report: &mut TossReport) {
        report.area_fix += 1;
        let supplied = message.subject.split_whitespace().next().unwrap_or_default();
        let expected = &self.config.links[index].area_fix_password;
        let mut results = Vec::new();
        if !expected.eq_ignore_ascii_case(supplied) {
            results.push("AreaFix password is incorrect.".to_string());
            self.area_fix_response(index, message, results);
            return;
        }

        let body = Kludges::split(&message.text).body;
        let mut changed = false;
        for line in body.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with("---")) {
            let command = line.to_ascii_uppercase();
            if command == "%LIST" || command == "%QUERY" {
                results.push(format!("Active areas: {}", self.link_areas[index].join(" ")));
                continue;
            }
            if command == "%HELP" {
                results.push("Commands: +TAG, -TAG, %+ALL, %-ALL, %LIST, %QUERY, %HELP".to_string());
                continue;
            }
            if command == "%+ALL" {
                // A tag the node already has may be a passthru area, which is not in `lookup`.
                let available: Vec<String> = self.lookup.keys().cloned().collect();
                for tag in available {
                    if !self.link_areas[index].iter().any(|area| area.eq_ignore_ascii_case(&tag)) {
                        self.link_areas[index].push(tag);
                        changed = true;
                    }
                }
                self.link_areas[index].sort();
                results.push("Added all available areas.".to_string());
                continue;
            }
            if command == "%-ALL" {
                changed |= !self.link_areas[index].is_empty();
                self.link_areas[index].clear();
                results.push("Removed all areas.".to_string());
                continue;
            }

            let (remove, tag) = match command.as_bytes().first() {
                Some(b'-') => (true, command[1..].trim()),
                Some(b'+') => (false, command[1..].trim()),
                _ => (false, command.trim()),
            };
            if tag.is_empty() {
                continue;
            }
            if remove {
                let before = self.link_areas[index].len();
                self.link_areas[index].retain(|area| !area.eq_ignore_ascii_case(tag));
                results.push(if self.link_areas[index].len() < before {
                    changed = true;
                    format!("Removed: {tag}")
                } else {
                    format!("Not active: {tag}")
                });
                continue;
            }

            let available = self.lookup.contains_key(tag);
            if available || self.config.options.pass_thru && self.config.options.auto_add_passthru {
                if !self.link_areas[index].iter().any(|area| area.eq_ignore_ascii_case(tag)) {
                    self.link_areas[index].push(tag.to_string());
                    self.link_areas[index].sort();
                    changed = true;
                }
                results.push(format!("Added: {tag}"));
            } else if self.config.options.area_fix_forwarding {
                if self.forward_area_fix(index, message, tag) {
                    results.push(format!("Forwarded request: {tag}"));
                } else {
                    results.push(format!("Cannot forward: {tag}; no upstream node is configured."));
                }
            } else {
                results.push(format!("Area not available: {tag}"));
            }
        }
        if changed {
            report.link_updates.insert(index, self.link_areas[index].clone());
        }
        self.area_fix_response(index, message, results);
    }

    fn forward_area_fix(&mut self, requester: usize, message: &PackedMessage, tag: &str) -> bool {
        // A node without a host cannot be called, so it cannot answer the request.
        let Some(upstream) = (0..self.config.links.len()).find(|index| *index != requester && !self.config.links[*index].host.is_empty()) else {
            return false;
        };
        let mut request = PackedMessage {
            to: "AREAFIX".to_string(),
            from: message.from.clone(),
            subject: self.config.links[upstream].area_fix_password.clone(),
            text: format!("+{tag}\r"),
            attributes: packet::attribute::PRIVATE,
            written: chrono::Local::now().naive_local(),
            ..Default::default()
        };
        request.dest = self.config.links[upstream].address;
        self.forward[upstream].push(request);
        true
    }

    fn area_fix_response(&mut self, index: usize, request: &PackedMessage, results: Vec<String>) {
        if !self.config.options.make_response {
            return;
        }
        self.forward[index].push(PackedMessage {
            to: request.from.clone(),
            from: "AreaFix".to_string(),
            subject: "AreaFix request results".to_string(),
            text: results.join("\r") + "\r",
            attributes: packet::attribute::PRIVATE,
            written: chrono::Local::now().naive_local(),
            dest: self.config.links[index].address,
            ..Default::default()
        });
    }

    fn import(&mut self, message: &PackedMessage, path: &Path, echo: bool, report: &mut TossReport) -> Res<()> {
        let kludges = Kludges::split(&message.text);
        if self.config.options.check_dupe_path && self.travelled_here(&kludges.path) {
            report.duplicates += 1;
            return Ok(());
        }
        let base = self.bases.get(path)?;
        if self.config.options.check_dupe_msg_id
            && let Some(id) = &kludges.msgid
        {
            let crc = JamMessageBase::crc(&BString::from(id.as_str()));
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
            .filter(|(index, link)| {
                self.link_areas[*index].iter().any(|area| area.eq_ignore_ascii_case(tag))
                    && (link.address.net, link.address.node) != (from.net, from.node)
                    && !seen.contains(&(link.address.net, link.address.node))
            })
            .map(|(index, _)| index)
            .collect();
        let Some(first) = takers.first() else {
            return false;
        };
        let Some(aka) = self.config.aka_for(&self.config.links[*first]).cloned() else {
            return false;
        };
        let addresses: Vec<EchomailAddress> = takers.iter().map(|index| self.config.links[*index].address).collect();
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
        for (index, packet) in std::mem::take(&mut self.routed) {
            let link = &self.config.links[index];
            report.bundles.push(deliver_packet(self.config, link, packet, &now)?);
        }
        Ok(())
    }
}

/// The field holds eight characters, and a mailer pads what it writes there
/// with spaces or with nuls.
fn packet_password_matches(configured: &str, received: &str) -> bool {
    let configured: String = configured.trim_end().chars().take(8).collect();
    configured.trim_end().eq_ignore_ascii_case(received.trim_end())
}

fn to_jam(message: &PackedMessage, kludges: &Kludges, echo: bool) -> JamMessage {
    let mut flags = if echo {
        attributes::MSG_TYPEECHO
    } else {
        attributes::MSG_TYPENET | attributes::MSG_PRIVATE
    };
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
            let mut ids: Vec<u32> = base
                .messages()
                .flatten()
                .map(|header| header.msgid_crc)
                .filter(|crc| *crc != NO_MSGID)
                .collect();
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
        return JamMessageBase::open(path).context(|| format!("Cannot open the message base {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context(|| format!("Cannot create the directory {} the message base belongs in", parent.display()))?;
    }
    JamMessageBase::create(path).context(|| format!("Cannot create the message base {}", path.display()))
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
        let text = fs::read_to_string(&path).context(|| format!("Cannot read {}, which holds how far the last scan got", path.display()))?;
        toml::from_str(&text).context(|| {
            format!(
                "Cannot read {}, which holds how far the last scan got. Deleting it makes the next scan start from where the bases stand now",
                path.display()
            )
        })
    }

    fn save(&self, config: &FtnConfig) -> Res<()> {
        fs::create_dir_all(&config.outbound).context(|| format!("Cannot create the outbound {}", config.outbound.display()))?;
        let path = Self::path(config);
        fs::write(&path, toml::to_string_pretty(self)?).context(|| format!("Cannot write {}, which holds how far this scan got", path.display()))?;
        Ok(())
    }
}

/// What one run over the message bases put into the outbound.
#[derive(Debug, Default)]
pub struct ScanReport {
    pub exported: usize,

    /// Netmail messages written here and packed for their destination.
    pub netmail: usize,

    /// Netmail that names no destination, and netmail no link can carry,
    /// by message number.
    pub unaddressed: Vec<u32>,
    pub undeliverable: Vec<(u32, String)>,

    /// Tags whose area is the netmail base, which must never leave as echomail.
    pub refused: Vec<String>,

    pub bundles: Vec<PathBuf>,
}

/// Packs everything written here since the last run into bundles for the links
/// that asked for the area it was written in.
pub fn scan_outbound(config: &FtnConfig, areas: &AreaMap, now: &NaiveDateTime) -> Res<ScanReport> {
    let mut report = ScanReport::default();
    if !config.options.enabled || !config.options.process_out || config.links.is_empty() {
        return Ok(report);
    }
    let mut state = ScanState::load(config)?;
    let mut waiting: Vec<Vec<PackedMessage>> = vec![Vec::new(); config.links.len()];

    for area in areas {
        let (tag, path) = (&area.tag, &area.path);
        if tag.is_empty() {
            continue;
        }
        // Netmail is addressed to one node, so handing it to everyone who
        // carries a tag would put private mail in front of every downlink.
        if *path == config.netmail || *path == config.bad_netmail {
            report.refused.push(tag.clone());
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
        let mut base = JamMessageBase::open(path).context(|| format!("Cannot open the message base {} carrying {}", path.display(), tag))?;
        let high = base.highest_message_number();
        let Some(last) = state.exported.get(tag).copied() else {
            // Nothing was ever sent out of this area, and handing a link the
            // whole history of a board it just met would be rude.
            state.exported.insert(tag.clone(), high);
            continue;
        };
        log::info!("Scanning {} from message {} through {}", tag, last.saturating_add(1), high);

        // The area is fed by whatever address answers for the first link that
        // carries it, because one message can only have one origin.
        let Some(aka) = config.aka_for(&config.links[subscribers[0]]).cloned() else {
            continue;
        };
        let origin = if area.origin.is_empty() {
            config.origin.as_str()
        } else {
            area.origin.as_str()
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
            let msgid = if let Some(id) = subfield(&header, SubfieldType::MsgID) {
                id
            } else {
                state.serial = state.serial.wrapping_add(1);
                let id = format!("{} {:08x}", aka.address, state.serial);
                header.msgid_crc = JamMessageBase::crc(&BString::from(id.as_str()));
                header.sub_fields.push(MessageSubfield::new(SubfieldType::MsgID, BString::from(id.as_str())));
                // The id is written back so that a reply arriving for it
                // still finds the message it belongs to.
                raw::update_header(&mut base, number, &header).context(|| format!("Cannot stamp message {number} of {tag} with a message id"))?;
                id
            };
            let text = base
                .read_message_text(&header)
                .context(|| format!("Cannot read message {number} of {tag} out of {}", path.display()))?;
            let message = PackedMessage {
                orig: aka.address,
                dest: EchomailAddress::default(),
                attributes: 0,
                cost: 0,
                written: chrono::DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_default().naive_utc(),
                to: header.to().map(std::string::ToString::to_string).unwrap_or_default(),
                from: header.from().map(std::string::ToString::to_string).unwrap_or_default(),
                subject: header.subject().map(std::string::ToString::to_string).unwrap_or_default(),
                text: exported_text(
                    tag,
                    &msgid,
                    subfield(&header, SubfieldType::ReplyID).as_deref(),
                    &text.to_string(),
                    origin,
                    &aka,
                    &subscribers.iter().map(|index| config.links[*index].address).collect::<Vec<_>>(),
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
        let destination = &config.links[index];
        let (queue_link, packet_dest) = if config.options.route_echo_mail {
            match config.route_for(&destination.address) {
                Some((_, via)) if config.options.re_address => (&config.links[via], config.links[via].address),
                Some((_, via)) => (&config.links[via], destination.address),
                None => (destination, destination.address),
            }
        } else {
            (destination, destination.address)
        };
        let Some(aka) = config.aka_for(queue_link) else {
            continue;
        };
        report
            .bundles
            .push(deliver_to(config, queue_link, destination.address, packet_dest, aka, messages, now)?);
    }

    scan_netmail(config, &mut state, &mut report, now)?;
    state.save(config)?;
    Ok(report)
}

/// The link a netmail leaves through: the node itself when it is one of ours,
/// the next hop when a route names one, and the first node we can call
/// otherwise, which for a point is its boss.
fn netmail_route(config: &FtnConfig, dest: &EchomailAddress) -> Option<usize> {
    if let Some(index) = config.links.iter().position(|link| link.address == *dest) {
        return Some(index);
    }
    if config.options.enable_routing
        && let Some((_, via)) = config.route_for(dest)
    {
        return Some(via);
    }
    let mut reachable = (0..config.links.len()).filter(|index| !config.links[*index].host.is_empty());
    let only = reachable.next()?;
    reachable.next().is_none().then_some(only)
}

/// Packs the netmail written here for the node it is addressed to. Everything
/// the tosser imported carries `MSG_TYPENET`, so what is left is what was
/// written on this board.
fn scan_netmail(config: &FtnConfig, state: &mut ScanState, report: &mut ScanReport, now: &NaiveDateTime) -> Res<()> {
    if !config.netmail.with_extension("jhr").exists() {
        return Ok(());
    }
    let mut base = JamMessageBase::open(&config.netmail).context(|| format!("Cannot open the netmail base {}", config.netmail.display()))?;

    let mut waiting: BTreeMap<usize, Vec<(u32, PackedMessage)>> = BTreeMap::new();
    for number in base.lowest_message_number()..=base.highest_message_number() {
        let Ok(mut header) = base.read_header(number) else {
            continue;
        };
        if header.is_deleted() || header.attributes & (attributes::MSG_TYPENET | attributes::MSG_SENT) != 0 {
            continue;
        }
        let Some(dest) = subfield(&header, SubfieldType::AddressD).and_then(|address| EchomailAddress::parse(&address)) else {
            report.unaddressed.push(number);
            continue;
        };
        let Some(index) = netmail_route(config, &dest) else {
            report.undeliverable.push((number, dest.to_string()));
            continue;
        };
        let Some(aka) = config.aka_for(&config.links[index]).cloned() else {
            report.undeliverable.push((number, dest.to_string()));
            continue;
        };

        let msgid = if let Some(id) = subfield(&header, SubfieldType::MsgID) {
            id
        } else {
            state.serial = state.serial.wrapping_add(1);
            let id = format!("{} {:08x}", aka.address, state.serial);
            header.msgid_crc = JamMessageBase::crc(&BString::from(id.as_str()));
            header.sub_fields.push(MessageSubfield::new(SubfieldType::MsgID, BString::from(id.as_str())));
            raw::update_header(&mut base, number, &header).context(|| format!("Cannot stamp netmail {number} with a message id"))?;
            id
        };
        let text = base
            .read_message_text(&header)
            .context(|| format!("Cannot read netmail {number} out of {}", config.netmail.display()))?;

        let mut attributes = packet::attribute::PRIVATE;
        for (jam, packed) in [
            (attributes::MSG_CRASH, packet::attribute::CRASH),
            (attributes::MSG_FILEATTACH, packet::attribute::FILE_ATTACHED),
            (attributes::MSG_FILEREQUEST, packet::attribute::FILE_REQUEST),
            (attributes::MSG_RECEIPTREQ, packet::attribute::RETURN_RECEIPT_REQUEST),
            (attributes::MSG_KILLSENT, packet::attribute::KILL_SENT),
        ] {
            if header.attributes & jam != 0 {
                attributes |= packed;
            }
        }

        waiting.entry(index).or_default().push((
            number,
            PackedMessage {
                orig: aka.address,
                dest,
                attributes,
                cost: 0,
                written: chrono::DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_default().naive_utc(),
                to: header.to().map(std::string::ToString::to_string).unwrap_or_default(),
                from: header.from().map(std::string::ToString::to_string).unwrap_or_default(),
                subject: header.subject().map(std::string::ToString::to_string).unwrap_or_default(),
                text: netmail_text(&msgid, subfield(&header, SubfieldType::ReplyID).as_deref(), &text.to_string()),
            },
        ));
    }

    for (index, messages) in waiting {
        let link = &config.links[index];
        let Some(aka) = config.aka_for(link) else {
            continue;
        };
        let mut packet = Packet::new(PacketHeader::new(aka.address, link.address, *now, &link.packet_password));
        packet.messages = messages.iter().map(|(_, message)| message.clone()).collect();
        report.bundles.push(deliver_packet(config, link, packet, now)?);

        for (number, _) in messages {
            let Ok(mut header) = base.read_header(number) else {
                continue;
            };
            // A node that asked for its mail back gets it once and then it goes.
            if header.attributes & attributes::MSG_KILLSENT != 0 {
                base.delete_message(number)
                    .context(|| format!("Cannot remove netmail {number} after sending it"))?;
            } else {
                header.attributes |= attributes::MSG_SENT;
                raw::update_header(&mut base, number, &header).context(|| format!("Cannot mark netmail {number} as sent"))?;
            }
            report.netmail += 1;
        }
    }
    Ok(())
}

/// Netmail names no area and travels no echo, so it carries neither an
/// `AREA:` line nor a seen-by trail.
fn netmail_text(msgid: &str, reply: Option<&str>, body: &str) -> String {
    let mut text = format!("\x01MSGID: {msgid}\r");
    if let Some(reply) = reply {
        let _ = write!(text, "\x01REPLY: {reply}\r");
    }
    let _ = write!(text, "\x01PID: {}\r", product());
    text.push_str(&body.replace("\r\n", "\n").replace('\n', "\r"));
    if !text.ends_with('\r') {
        text.push('\r');
    }
    text
}

/// Puts what is waiting for one link into a bundle of its own.
fn deliver(config: &FtnConfig, link: &FtnLink, aka: &FtnAka, messages: Vec<PackedMessage>, now: &NaiveDateTime) -> Res<PathBuf> {
    deliver_to(config, link, link.address, link.address, aka, messages, now)
}

fn deliver_to(
    config: &FtnConfig,
    queue_link: &FtnLink,
    message_dest: EchomailAddress,
    packet_dest: EchomailAddress,
    aka: &FtnAka,
    mut messages: Vec<PackedMessage>,
    now: &NaiveDateTime,
) -> Res<PathBuf> {
    let directory = config.outbound_for(queue_link);
    fs::create_dir_all(&directory).context(|| format!("Cannot create the outbound {} of {}", directory.display(), queue_link.to_5d()))?;

    for message in &mut messages {
        message.orig = aka.address;
        message.dest = message_dest;
    }
    let mut packet = Packet::new(PacketHeader::new(aka.address, packet_dest, *now, &queue_link.packet_password));
    packet.messages = messages;

    deliver_packet(config, queue_link, packet, now)
}

fn deliver_packet(config: &FtnConfig, queue_link: &FtnLink, packet: Packet, now: &NaiveDateTime) -> Res<PathBuf> {
    let directory = config.outbound_for(queue_link);
    fs::create_dir_all(&directory).context(|| format!("Cannot create the outbound {} of {}", directory.display(), queue_link.to_5d()))?;

    let work = tempfile::tempdir_in(&directory).context(|| format!("Cannot create a working directory in the outbound {}", directory.display()))?;
    let written = work.path().join(bundle::packet_name(now));
    packet
        .save(&written)
        .context(|| format!("Cannot pack the mail waiting for {}", queue_link.to_5d()))?;
    let name = bundle::next_bundle(&directory, &packet.header.orig, &queue_link.address, now)?;
    bundle::pack(&[written], &name).context(|| format!("Cannot bundle the mail waiting for {}", queue_link.to_5d()))?;
    Ok(name)
}

fn subfield(header: &jamjam::jam::msg_header::JamMessageHeader, kind: SubfieldType) -> Option<String> {
    header
        .sub_fields
        .iter()
        .find(|field| field.field_type() == kind)
        .map(|field| field.content().to_string())
}

/// Builds what the other side gets to see: the area, the kludges that say where
/// the message came from, the message itself, and the trail it has travelled.
fn exported_text(tag: &str, msgid: &str, reply: Option<&str>, body: &str, origin: &str, aka: &FtnAka, links: &[EchomailAddress]) -> String {
    let mut text = format!("AREA:{}\r\x01MSGID: {}\r", tag.to_uppercase(), msgid);
    if let Some(reply) = reply {
        let _ = write!(text, "\x01REPLY: {reply}\r");
    }
    let _ = write!(text, "\x01PID: {}\r", product());
    text.push_str(&body.replace("\r\n", "\n").replace('\n', "\r"));
    if !text.ends_with('\r') {
        text.push('\r');
    }
    if !body.contains(" * Origin:") {
        let _ = write!(text, "\r--- {}\r * Origin: {} ({})\r", product(), origin, aka.address);
    }

    let mut seen: Vec<(u16, u16)> = links.iter().map(|link| (link.net, link.node)).collect();
    seen.push((aka.address.net, aka.address.node));
    seen.sort_unstable();
    seen.dedup();
    for line in fold(&seen.iter().map(|(net, node)| format!("{net}/{node}")).collect::<Vec<_>>()) {
        let _ = write!(text, "SEEN-BY: {line}\r");
    }
    let _ = write!(text, "\x01PATH: {}/{}\r", aka.address.net, aka.address.node);
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
    for line in fold(&seen.iter().map(|(net, node)| format!("{net}/{node}")).collect::<Vec<_>>()) {
        let _ = write!(out, "SEEN-BY: {line}\r");
    }
    let _ = write!(out, "\x01PATH: {}/{}\r", aka.address.net, aka.address.node);
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
        toss_inbound(config, areas).unwrap()
    }

    fn message(area: &str, text: &str) -> PackedMessage {
        PackedMessage {
            orig: address("21:1/1"),
            dest: address("21:1/100"),
            written: when(),
            to: "All".to_string(),
            from: "Someone".to_string(),
            subject: "Hello".to_string(),
            text: format!("AREA:{area}\r{text}"),
            ..Default::default()
        }
    }

    fn drop_packet(config: &FtnConfig, messages: Vec<PackedMessage>) {
        drop_packet_from(config, "21:1/1", messages);
    }

    fn drop_packet_from(config: &FtnConfig, source: &str, messages: Vec<PackedMessage>) {
        drop_packet_from_with_password(config, source, "", messages);
    }

    fn drop_packet_from_with_password(config: &FtnConfig, source: &str, password: &str, messages: Vec<PackedMessage>) {
        let mut packet = Packet::new(PacketHeader::new(address(source), address("21:1/100"), when(), password));
        packet.messages = messages;
        fs::create_dir_all(&config.inbound).unwrap();
        packet.save(&config.inbound.join(bundle::packet_name(&when()))).unwrap();
    }

    #[test]
    fn test_a_message_from_a_packet_lands_in_the_area_that_carries_its_tag() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("bases/general"))];
        drop_packet(&config, vec![message("FSX_GEN", "\x01MSGID: 21:1/2 11223344\rBody\r")]);

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 1);
        assert!(report.failed.is_empty());
        let base = JamMessageBase::open(&areas[0].path).unwrap();
        let header = base.read_header(1).unwrap();
        assert_eq!(base.read_message_text(&header).unwrap().to_string(), "Body");
        assert_eq!(header.from().unwrap().to_string(), "Someone");
        assert_eq!(subfield(&header, SubfieldType::MsgID).unwrap(), "21:1/2 11223344");
        assert!(fs::read_dir(&config.inbound).unwrap().next().is_none());
    }

    #[test]
    fn test_disabled_fido_processing_leaves_inbound_untouched() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.enabled = false;
        drop_packet(&config, vec![message("FSX_GEN", "Body\r")]);
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("general"))];

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 0);
        assert!(fs::read_dir(&config.inbound).unwrap().next().is_some());
        assert!(!areas[0].path.with_extension("jhr").exists());
    }

    #[test]
    fn test_the_same_message_id_is_only_imported_once() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("bases/general"))];
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
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("bases/general"))];
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
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("bases/general"))];
        drop_packet(&config, vec![message("FSX_GEN", "Body\r\x01PATH: 1/2 100\r")]);

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 0);
        assert_eq!(report.duplicates, 1);
    }

    #[test]
    fn test_a_packet_for_another_system_is_left_alone_unless_orphans_are_wanted() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("bases/general"))];
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
    fn test_a_packet_for_a_route_is_queued_for_its_next_hop() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.enable_routing = true;
        config.routes.push(super::super::FtnRoute {
            destination: address("21:1/200"),
            via: address("21:1/1"),
        });
        let packet = Packet::new(PacketHeader::new(address("21:1/2"), address("21:1/200"), when(), ""));
        fs::create_dir_all(&config.inbound).unwrap();
        packet.save(&config.inbound.join(bundle::packet_name(&when()))).unwrap();

        let report = toss(&config, &[]);

        assert_eq!(report.routed, 1);
        assert_eq!(report.bundles.len(), 1);
        assert!(report.bundles[0].starts_with(config.outbound_for(&config.links[0])));
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        assert_eq!(Packet::load(&packets[0]).unwrap().header.dest, address("21:1/200"));
    }

    #[test]
    fn test_readdressed_route_names_the_next_hop_in_the_packet_header() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.enable_routing = true;
        config.options.re_address = true;
        config.routes.push(super::super::FtnRoute {
            destination: address("21:1/200"),
            via: address("21:1/1"),
        });
        let packet = Packet::new(PacketHeader::new(address("21:1/2"), address("21:1/200"), when(), ""));
        fs::create_dir_all(&config.inbound).unwrap();
        packet.save(&config.inbound.join(bundle::packet_name(&when()))).unwrap();

        let report = toss(&config, &[]);

        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let routed = Packet::load(&packets[0]).unwrap();
        assert_eq!(routed.header.orig, address("21:1/100"));
        assert_eq!(routed.header.dest, address("21:1/1"));
    }

    #[test]
    fn test_a_route_is_not_sent_straight_back_to_the_packet_source() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.enable_routing = true;
        config.routes.push(super::super::FtnRoute {
            destination: address("21:1/200"),
            via: address("21:1/1"),
        });
        let packet = Packet::new(PacketHeader::new(address("21:1/1"), address("21:1/200"), when(), ""));
        fs::create_dir_all(&config.inbound).unwrap();
        packet.save(&config.inbound.join(bundle::packet_name(&when()))).unwrap();

        let report = toss(&config, &[]);

        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].1.contains("straight back"));
        assert!(fs::read_dir(&config.inbound).unwrap().next().is_some());
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
    fn test_netmail_from_an_unconfigured_node_is_kept_apart_on_a_secure_board() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.secure = true;
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Who are you\r".to_string();
        netmail.to = "The Sysop".to_string();
        drop_packet_from(&config, "21:1/2", vec![netmail]);

        let report = toss_inbound(&config, &[]).unwrap();

        assert_eq!(report.netmail, 1);
        assert_eq!(report.untrusted_netmail.get("21:1/2"), Some(&1));
        assert_eq!(JamMessageBase::open(&config.bad_netmail).unwrap().active_messages(), 1);
    }

    #[test]
    fn test_secure_netmail_from_a_configured_link_goes_to_netmail() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.secure = true;
        config.links[0].packet_password = "Correct Password".to_string();
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Hello\r".to_string();
        drop_packet_from_with_password(&config, "21:1/1", "correct ", vec![netmail]);

        let report = toss(&config, &[]);

        assert!(report.untrusted_netmail.is_empty());
        assert_eq!(JamMessageBase::open(&config.netmail).unwrap().active_messages(), 1);
        assert!(!config.bad_netmail.exists());
    }

    #[test]
    fn test_a_configured_links_wrong_packet_password_leaves_the_packet_in_inbound() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.links[0].packet_password = "correct password".to_string();
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Hello\r".to_string();
        drop_packet_from_with_password(&config, "21:1/1", "wrong", vec![netmail]);

        let report = toss(&config, &[]);

        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].1.contains("packet password"), "{}", report.failed[0].1);
        assert!(fs::read_dir(&config.inbound).unwrap().next().is_some());
        assert!(!config.netmail.exists());
    }

    #[test]
    fn test_a_packet_password_padded_out_to_the_field_still_matches() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.links[0].packet_password = "secret".to_string();
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Hello\r".to_string();
        drop_packet_from_with_password(&config, "21:1/1", "secret  ", vec![netmail]);

        let report = toss(&config, &[]);

        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(JamMessageBase::open(&config.netmail).unwrap().active_messages(), 1);
    }

    #[test]
    fn test_a_link_without_a_packet_password_takes_the_packets_as_they_come() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        // The session password is not the packet password, so it must not be checked.
        config.links[0].password = "binkp secret".to_string();
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Hello\r".to_string();
        drop_packet_from_with_password(&config, "21:1/1", "", vec![netmail]);

        let report = toss(&config, &[]);

        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(JamMessageBase::open(&config.netmail).unwrap().active_messages(), 1);
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
        assert!(base.read_header(1).unwrap().is_private());
    }

    #[test]
    fn test_areafix_changes_a_nodes_subscription_and_sends_a_response() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.make_response = true;
        config.links[0].areas.clear();
        config.links[0].area_fix_password = "secret".to_string();
        let mut request = message("FSX_GEN", "");
        request.to = "AreaFix".to_string();
        request.from = "Remote Sysop".to_string();
        request.subject = "secret".to_string();
        request.text = "+FSX_GEN\r%LIST\r".to_string();
        drop_packet(&config, vec![request]);
        let areas = vec![EchoArea::new("FSX_GEN", directory.path().join("general"))];

        let report = toss(&config, &areas);

        assert_eq!(report.area_fix, 1);
        assert_eq!(report.link_updates.get(&0), Some(&vec!["FSX_GEN".to_string()]));
        assert_eq!(report.bundles.len(), 1);
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let response = Packet::load(&packets[0]).unwrap();
        assert_eq!(response.messages[0].from, "AreaFix");
        assert_eq!(response.messages[0].to, "Remote Sysop");
        assert!(response.messages[0].text.contains("Added: FSX_GEN"));
    }

    #[test]
    fn test_areafix_rejects_the_wrong_password_without_changing_subscriptions() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.links[0].areas.clear();
        config.links[0].area_fix_password = "secret".to_string();
        let mut request = message("FSX_GEN", "");
        request.to = "AreaFix".to_string();
        request.subject = "wrong".to_string();
        request.text = "+FSX_GEN\r".to_string();
        drop_packet(&config, vec![request]);

        let report = toss(&config, &[EchoArea::new("FSX_GEN", directory.path().join("general"))]);

        assert_eq!(report.area_fix, 1);
        assert!(report.link_updates.is_empty());
        assert!(report.bundles.is_empty());
    }

    #[test]
    fn test_areafix_can_auto_add_an_unknown_passthru_area() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.pass_thru = true;
        config.options.auto_add_passthru = true;
        config.links[0].areas.clear();
        let mut request = message("FSX_GEN", "");
        request.to = "AreaFix".to_string();
        request.subject.clear();
        request.text = "+NEW_ECHO\r".to_string();
        drop_packet(&config, vec![request]);

        let report = toss(&config, &[]);

        assert_eq!(report.link_updates.get(&0), Some(&vec!["NEW_ECHO".to_string()]));
    }

    #[test]
    fn test_areafix_add_all_keeps_the_passthru_areas_a_node_already_had() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.links[0].areas = vec!["PASSTHRU_ONLY".to_string()];
        let mut request = message("FSX_GEN", "");
        request.to = "AreaFix".to_string();
        request.subject.clear();
        request.text = "%+ALL\r".to_string();
        drop_packet(&config, vec![request]);

        let report = toss(&config, &[EchoArea::new("FSX_GEN", directory.path().join("general"))]);

        assert_eq!(report.link_updates.get(&0), Some(&vec!["FSX_GEN".to_string(), "PASSTHRU_ONLY".to_string()]));
    }

    #[test]
    fn test_an_areafix_query_alone_does_not_rewrite_the_subscription() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.make_response = true;
        let mut request = message("FSX_GEN", "");
        request.to = "AreaFix".to_string();
        request.subject.clear();
        request.text = "%LIST\r".to_string();
        drop_packet(&config, vec![request]);

        let report = toss(&config, &[EchoArea::new("FSX_GEN", directory.path().join("general"))]);

        assert_eq!(report.area_fix, 1);
        assert!(report.link_updates.is_empty());
        assert_eq!(report.bundles.len(), 1);
    }

    #[test]
    fn test_areafix_forwards_an_unknown_area_to_the_first_other_node() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.area_fix_forwarding = true;
        config.links[0].areas.clear();
        config.links.push(FtnLink {
            address: address("21:1/2"),
            domain: "fsxnet".to_string(),
            host: "uplink.example".to_string(),
            area_fix_password: "upstream-secret".to_string(),
            ..Default::default()
        });
        let mut request = message("FSX_GEN", "");
        request.to = "AreaFix".to_string();
        request.subject.clear();
        request.text = "+NEW_ECHO\r".to_string();
        drop_packet(&config, vec![request]);

        let report = toss(&config, &[]);

        assert_eq!(report.bundles.len(), 1);
        assert!(report.bundles[0].starts_with(config.outbound_for(&config.links[1])));
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let forwarded = Packet::load(&packets[0]).unwrap();
        assert_eq!(forwarded.messages[0].to, "AREAFIX");
        assert_eq!(forwarded.messages[0].subject, "upstream-secret");
        assert!(forwarded.messages[0].text.ends_with("+NEW_ECHO\r"));
    }

    #[test]
    fn test_sysop_is_changed_to_fido_sysop_on_import() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let echo_path = directory.path().join("general");
        let areas = vec![EchoArea::new("FSX_GEN", echo_path.clone())];
        let mut echo = message("FSX_GEN", "Body\r");
        echo.to = "Sysop".to_string();
        let mut netmail = message("FSX_GEN", "");
        netmail.to = "Sysop".to_string();
        netmail.text = "Private\r".to_string();
        drop_packet(&config, vec![echo, netmail]);

        let report = toss(&config, &areas);

        assert_eq!(report.imported, 1);
        assert_eq!(report.netmail, 1);
        assert_eq!(JamMessageBase::open(&echo_path).unwrap().read_header(1).unwrap().to().unwrap(), b"FIDO_SYSOP");
        assert_eq!(
            JamMessageBase::open(&config.netmail).unwrap().read_header(1).unwrap().to().unwrap(),
            b"FIDO_SYSOP"
        );
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
        assert!(report.failed[0].1.contains("12345678.pkt"), "{}", report.failed[0].1);
        assert!(report.failed[0].1.contains("not all here"), "{}", report.failed[0].1);
    }

    #[test]
    fn test_a_broken_packet_inside_a_bundle_is_reported_under_the_name_it_arrived_as() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        fs::create_dir_all(&config.inbound).unwrap();
        let loose = directory.path().join("00000001.pkt");
        fs::write(&loose, b"not a packet at all").unwrap();
        let arrived = config.inbound.join("0000ff9d.su0");
        bundle::pack(&[loose], &arrived).unwrap();

        let report = toss(&config, &[]);

        assert_eq!(report.failed.len(), 1);
        let complaint = &report.failed[0].1;
        assert!(complaint.contains("00000001.pkt"), "{complaint}");
        assert!(complaint.contains("0000ff9d.su0"), "{complaint}");
        assert!(arrived.exists());
    }

    #[test]
    fn test_a_bundle_with_an_unexpected_file_is_kept_for_the_sysop() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        fs::create_dir_all(&config.inbound).unwrap();
        let unexpected = directory.path().join("readme.txt");
        fs::write(&unexpected, b"not packed mail").unwrap();
        let arrived = config.inbound.join("0000ff9d.su0");
        bundle::pack(&[unexpected], &arrived).unwrap();

        let report = toss(&config, &[]);

        assert!(report.failed.is_empty());
        assert!(arrived.exists());
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
        let areas = vec![EchoArea::new("FSX_GEN", path.clone())];

        let report = scan_outbound(&config, &areas, &when()).unwrap();
        assert_eq!(report.exported, 0);

        base.write_message(&JamMessage::default().with_text(BString::from("new"))).unwrap();
        base.write_jhr_header().unwrap();
        let report = scan_outbound(&config, &areas, &when()).unwrap();
        assert_eq!(report.exported, 1);
    }

    /// The address a netmail is going to is the one its reply carries, so the
    /// scanner reads it back out of the header the tosser wrote.
    fn netmail_for(dest: &str, killsent: bool) -> JamMessage {
        let mut flags = attributes::MSG_PRIVATE;
        if killsent {
            flags |= attributes::MSG_KILLSENT;
        }
        JamMessage::default()
            .with_from(BString::from("Sysop"))
            .with_to(BString::from("Remote Sysop"))
            .with_subject(BString::from("Re: Hello"))
            .with_attributes(flags)
            .with_text(BString::from("Answer"))
            .with_sub_field(MessageSubfield::new(SubfieldType::AddressD, BString::from(dest)))
    }

    #[test]
    fn test_netmail_written_here_is_packed_for_the_node_it_is_addressed_to() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&netmail_for("21:1/1", false)).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(report.netmail, 1);
        assert_eq!(report.bundles.len(), 1);
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let packet = Packet::load(&packets[0]).unwrap();
        let message = &packet.messages[0];
        assert_eq!(message.dest, address("21:1/1"));
        assert!(!message.is_echomail(), "{:?}", message.text);
        assert_eq!(message.attributes & packet::attribute::PRIVATE, packet::attribute::PRIVATE);
        assert!(message.text.contains("\x01MSGID: 21:1/100 "), "{:?}", message.text);
    }

    #[test]
    fn test_netmail_is_only_sent_once() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&netmail_for("21:1/1", false)).unwrap();
        base.write_jhr_header().unwrap();

        assert_eq!(scan_outbound(&config, &[], &when()).unwrap().netmail, 1);
        let again = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(again.netmail, 0);
        assert!(again.bundles.is_empty());
    }

    /// What the tosser imported carries `MSG_TYPENET`, and sending it back out
    /// would return every message to the node it came from.
    #[test]
    fn test_netmail_that_came_in_from_the_network_is_not_sent_back_out() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut netmail = message("FSX_GEN", "");
        netmail.text = "Hello\r".to_string();
        drop_packet_from(&config, "21:1/1", vec![netmail]);
        toss(&config, &[]);

        let report = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(report.netmail, 0);
        assert!(report.bundles.is_empty());
    }

    #[test]
    fn test_netmail_asking_to_be_killed_is_gone_once_it_is_sent() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&netmail_for("21:1/1", true)).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(report.netmail, 1);
        assert_eq!(JamMessageBase::open(&config.netmail).unwrap().active_messages(), 0);
    }

    #[test]
    fn test_netmail_without_a_destination_stays_where_it_is() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("Nowhere"))).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(report.unaddressed, vec![1]);
        assert!(report.bundles.is_empty());
    }

    /// A point hands everything to its boss, so mail for a stranger still goes.
    #[test]
    fn test_netmail_for_a_node_we_do_not_know_leaves_through_the_first_link() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&netmail_for("21:99/99", false)).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(report.netmail, 1);
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let packet = Packet::load(&packets[0]).unwrap();
        assert_eq!(packet.header.dest, address("21:1/1"));
        assert_eq!(packet.messages[0].dest, address("21:99/99"));
    }

    #[test]
    fn test_netmail_for_an_unknown_node_needs_a_route_when_several_links_are_reachable() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.links.push(FtnLink {
            address: address("21:2/1"),
            domain: "fsxnet".to_string(),
            host: "other.example".to_string(),
            ..Default::default()
        });
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&netmail_for("21:99/99", false)).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &[], &when()).unwrap();

        assert_eq!(report.netmail, 0);
        assert_eq!(report.undeliverable, vec![(1, "21:99/99".to_string())]);
        assert!(report.bundles.is_empty());
    }

    /// Handing the netmail base a tag would put private mail in a bundle for
    /// every downlink that carries it.
    #[test]
    fn test_an_area_pointing_at_the_netmail_base_is_never_exported_as_echomail() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let mut base = open_base(&config.netmail).unwrap();
        base.write_message(&netmail_for("21:1/1", false)).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &[EchoArea::new("FSX_GEN", config.netmail.clone())], &when()).unwrap();

        assert_eq!(report.refused, vec!["FSX_GEN".to_string()]);
        assert_eq!(report.exported, 0);
    }

    #[test]
    fn test_disabled_fido_processing_does_not_scan_outbound() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.enabled = false;
        let path = directory.path().join("bases/general");
        let areas = vec![EchoArea::new("FSX_GEN", path.clone())];
        let mut base = open_base(&path).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("first"))).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &areas, &when()).unwrap();

        assert_eq!(report.exported, 0);
        assert!(report.bundles.is_empty());
        assert!(!config.outbound.exists());
    }

    #[test]
    fn test_a_message_written_here_leaves_as_a_bundle_for_the_link_that_asked_for_it() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let path = directory.path().join("bases/general");
        let mut base = open_base(&path).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("first"))).unwrap();
        base.write_jhr_header().unwrap();
        let areas = vec![EchoArea::new("FSX_GEN", path.clone())];
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
        assert!(text.starts_with("AREA:FSX_GEN\r"), "{text:?}");
        assert!(text.contains("\x01MSGID: 21:1/100 "), "{text:?}");
        assert!(text.contains(" * Origin: A board (21:1/100)\r"), "{text:?}");
        assert!(text.contains("SEEN-BY: 1/1 1/100\r"), "{text:?}");
        assert!(text.contains("\x01PATH: 1/100\r"), "{text:?}");
    }

    #[test]
    fn test_an_areas_own_origin_takes_the_place_of_the_board_wide_one() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let path = directory.path().join("bases/general");
        let mut base = open_base(&path).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("first"))).unwrap();
        base.write_jhr_header().unwrap();
        let mut areas = vec![EchoArea::new("FSX_GEN", path.clone())];
        areas[0].origin = "Just this area".to_string();
        scan_outbound(&config, &areas, &when()).unwrap();

        base.write_message(&JamMessage::default().with_text(BString::from("Body"))).unwrap();
        base.write_jhr_header().unwrap();
        let report = scan_outbound(&config, &areas, &when()).unwrap();

        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        let text = &Packet::load(&packets[0]).unwrap().messages[0].text;
        assert!(text.contains(" * Origin: Just this area (21:1/100)\r"), "{text:?}");
    }

    #[test]
    fn test_outbound_echomail_can_be_routed_through_a_next_hop() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.route_echo_mail = true;
        config.links.push(FtnLink {
            address: address("21:1/2"),
            domain: "fsxnet".to_string(),
            host: "target.example".to_string(),
            areas: vec!["FSX_GEN".to_string()],
            ..Default::default()
        });
        config.links[0].areas.clear();
        config.routes.push(super::super::FtnRoute {
            destination: address("21:1/2"),
            via: address("21:1/1"),
        });
        let path = directory.path().join("bases/general");
        let areas = vec![EchoArea::new("FSX_GEN", path.clone())];
        let mut base = open_base(&path).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("first"))).unwrap();
        base.write_jhr_header().unwrap();
        scan_outbound(&config, &areas, &when()).unwrap();
        base.write_message(&JamMessage::default().with_text(BString::from("second"))).unwrap();
        base.write_jhr_header().unwrap();

        let report = scan_outbound(&config, &areas, &when()).unwrap();

        assert_eq!(report.exported, 1);
        assert!(report.bundles[0].starts_with(config.outbound_for(&config.links[0])));
        let unpacked = tempfile::tempdir().unwrap();
        let packets = bundle::unpack(&report.bundles[0], unpacked.path()).unwrap();
        assert_eq!(Packet::load(&packets[0]).unwrap().header.dest, address("21:1/2"));
    }

    #[test]
    fn test_what_came_in_from_the_network_is_not_sent_back_out() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let path = directory.path().join("bases/general");
        let areas = vec![EchoArea::new("FSX_GEN", path.clone())];
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
