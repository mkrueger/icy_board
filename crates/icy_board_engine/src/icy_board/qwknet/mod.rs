use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};

use bstr::BString;
use chrono::{DateTime, Utc};
use jamjam::{
    jam::{
        JamMessage, JamMessageBase, attributes,
        msg_header::{MessageSubfield, SubfieldType},
    },
    qwk::qwk_message::{MSG_ACTIVE, QwkMessage},
};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

use super::{IcyBoardSerializer, get_path};
use crate::Res;

/// What a JAM header carries when it has no message id.
const NO_MSGID: u32 = 0xffff_ffff;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct QwkNetworkConfig {
    #[serde(default)]
    pub enabled: bool,

    /// The unique two-to-eight character `QWKnet` ID of this board.
    #[serde(default)]
    pub local_id: String,

    #[serde(default = "QwkNetworkConfig::default_inbound")]
    pub inbound: PathBuf,

    #[serde(default = "QwkNetworkConfig::default_outbound")]
    pub outbound: PathBuf,

    #[serde(rename = "hub", default)]
    pub hubs: Vec<QwkHub>,
}

impl QwkNetworkConfig {
    fn default_inbound() -> PathBuf {
        PathBuf::from("qwknet/inbound")
    }

    fn default_outbound() -> PathBuf {
        PathBuf::from("qwknet/outbound")
    }

    pub fn hub(&self, id: &str) -> Option<&QwkHub> {
        self.hubs.iter().find(|hub| hub.id.eq_ignore_ascii_case(id))
    }

    pub fn validate(&self) -> Res<()> {
        validate_id(&self.local_id)?;
        let mut ids = HashSet::new();
        for hub in &self.hubs {
            validate_id(&hub.id)?;
            if !ids.insert(hub.id.to_ascii_uppercase()) {
                return Err(format!("QWKnet hub {} is configured more than once", hub.id).into());
            }
            let mut conferences = HashSet::new();
            for area in &hub.areas {
                if area.remote_conference == 0 {
                    return Err(format!("QWKnet hub {} uses reserved conference 0 for {}", hub.id, area.local_area.display()).into());
                }
                if !conferences.insert(area.remote_conference) {
                    return Err(format!("QWKnet hub {} maps conference {} more than once", hub.id, area.remote_conference).into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct QwkHub {
    /// The QWK ID used by the remote hub, for example `VERT`.
    pub id: String,

    #[serde(default)]
    pub host: String,

    #[serde(default)]
    pub username: String,

    #[serde(default)]
    pub password: String,

    #[serde(default)]
    pub poll_minutes: u32,

    #[serde(rename = "area", default)]
    pub areas: Vec<QwkHubArea>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct QwkHubArea {
    /// Conference number used in packets exchanged with this hub.
    pub remote_conference: u16,

    /// JAM message base path, relative to the board directory when necessary.
    pub local_area: PathBuf,

    #[serde(default)]
    pub read_only: bool,
}

impl IcyBoardSerializer for QwkNetworkConfig {
    const FILE_TYPE: &'static str = "qwknet";
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ScanState {
    /// Keyed by hub conference, so renaming a local area cannot reset a pointer
    /// and send the whole base out again.
    #[serde(default)]
    conferences: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    pub messages: usize,
    pub packet: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TossReport {
    pub imported: usize,
    pub duplicates: usize,
    pub loops: usize,
    pub unknown_conferences: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PollReport {
    pub uploaded: bool,
    pub downloaded: Option<PathBuf>,
}

pub async fn poll(config: &QwkNetworkConfig, board_root: &Path, hub_id: &str) -> Res<PollReport> {
    config.validate()?;
    let hub = config.hub(hub_id).ok_or_else(|| format!("No QWKnet hub named {hub_id} is configured"))?;
    if hub.host.is_empty() {
        return Err(format!("QWKnet hub {} has no host", hub.id).into());
    }
    let username = if hub.username.is_empty() { &config.local_id } else { &hub.username };
    let endpoint = if hub.host.starts_with("http://") || hub.host.starts_with("https://") {
        format!("{}/qwk.ssjs", hub.host.trim_end_matches('/'))
    } else {
        format!("https://{}/qwk.ssjs", hub.host.trim_end_matches('/'))
    };
    let client = reqwest::Client::new();
    let outbound = get_path(board_root, &config.outbound).join(format!("{}.rep", hub.id));
    let mut report = PollReport::default();
    if outbound.is_file() {
        let response = client
            .post(&endpoint)
            .basic_auth(username, Some(&hub.password))
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(fs::read(&outbound)?)
            .send()
            .await?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(format!("QWKnet hub {} rejected {}: {}", hub.id, outbound.display(), response.status()).into());
        }
        fs::remove_file(&outbound)?;
        report.uploaded = true;
    }

    let response = client.get(&endpoint).basic_auth(username, Some(&hub.password)).send().await?;
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(report);
    }
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("QWKnet hub {} download failed: {}", hub.id, response.status()).into());
    }
    let data = response.bytes().await?;
    validate_qwk_archive(&data, &hub.id)?;
    let inbound = get_path(board_root, &config.inbound);
    fs::create_dir_all(&inbound)?;
    let path = next_packet_name(&inbound, &hub.id);
    let temporary = path.with_extension("qwk.tmp");
    fs::write(&temporary, &data)?;
    fs::rename(temporary, &path)?;

    let acknowledged = client
        .post(format!("{endpoint}?received={}", data.len()))
        .basic_auth(username, Some(&hub.password))
        .body(Vec::new())
        .send()
        .await?;
    if acknowledged.status() != reqwest::StatusCode::NO_CONTENT {
        return Err(format!("QWKnet hub {} did not acknowledge the downloaded packet: {}", hub.id, acknowledged.status()).into());
    }
    report.downloaded = Some(path);
    Ok(report)
}

pub fn scan(config: &QwkNetworkConfig, board_root: &Path, hub_id: &str) -> Res<ScanReport> {
    config.validate()?;
    let hub = config.hub(hub_id).ok_or_else(|| format!("No QWKnet hub named {hub_id} is configured"))?;
    let outbound = get_path(board_root, &config.outbound);
    fs::create_dir_all(&outbound)?;
    let packet = outbound.join(format!("{}.rep", hub.id));
    if packet.is_file() {
        return Ok(ScanReport {
            packet: Some(packet),
            ..Default::default()
        });
    }

    let state_path = outbound.join(format!("{}.state.toml", hub.id));
    let mut state = load_state(&state_path)?;
    let mut messages = Vec::new();
    let mut next_state = state.clone();
    for area in &hub.areas {
        if area.read_only {
            continue;
        }
        let path = get_path(board_root, &area.local_area);
        if !path.with_extension("jhr").is_file() {
            continue;
        }
        let base = JamMessageBase::open(&path)?;
        let key = area.remote_conference.to_string();
        let first = state
            .conferences
            .get(&key)
            .copied()
            .unwrap_or(0)
            .saturating_add(1)
            .max(base.lowest_message_number());
        let highest = base.highest_message_number();
        for number in first..=highest {
            let Ok(header) = base.read_header(number) else { continue };
            if header.is_deleted() || !written_here(header.attributes) {
                continue;
            }
            let text = base.read_message_text(&header)?;
            let msgid = subfield(&header, SubfieldType::MsgID).unwrap_or_else(|| {
                format!(
                    "<{}.{}.{}@{}>",
                    JamMessageBase::crc(&BString::from(path.to_string_lossy().as_ref())),
                    number,
                    header.date_written,
                    config.local_id
                )
            });
            let reply = subfield(&header, SubfieldType::ReplyID);
            let text = to_packet_text(&outbound_text(&msgid, reply.as_deref(), &text.to_string()));
            let date = DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_else(Utc::now);
            messages.push(QwkMessage {
                status: b' ',
                msg_number: area.remote_conference as u32,
                date_time: date.format("%m-%d-%y%H:%M").to_string().into(),
                to: header.to().cloned().unwrap_or_default(),
                from: header.from().cloned().unwrap_or_default(),
                subj: header.subject().cloned().unwrap_or_default(),
                password: BString::default(),
                ref_msg_number: header.reply_to,
                active_flag: MSG_ACTIVE,
                conference_number: area.remote_conference,
                logical_message_number: messages.len().saturating_add(1) as u16,
                net_tag: b'*',
                text: text.into(),
            });
        }
        next_state.conferences.insert(key, highest);
    }

    if messages.is_empty() {
        return Ok(ScanReport::default());
    }
    write_rep(&packet, &hub.id, &messages)?;
    state = next_state;
    save_state(&state_path, &state)?;
    Ok(ScanReport {
        messages: messages.len(),
        packet: Some(packet),
    })
}

pub fn toss(config: &QwkNetworkConfig, board_root: &Path, hub_id: &str, packet: &Path) -> Res<TossReport> {
    config.validate()?;
    let hub = config.hub(hub_id).ok_or_else(|| format!("No QWKnet hub named {hub_id} is configured"))?;
    let mut archive = zip::ZipArchive::new(fs::File::open(packet)?)?;
    let entry = (0..archive.len())
        .find(|index| archive.by_index(*index).is_ok_and(|file| file.name().eq_ignore_ascii_case("messages.dat")))
        .ok_or_else(|| format!("{} contains no MESSAGES.DAT", packet.display()))?;
    let mut data = Vec::new();
    archive.by_index(entry)?.read_to_end(&mut data)?;
    if data.len() < QwkMessage::HEADER_SIZE {
        return Err(format!("{} has a truncated MESSAGES.DAT", packet.display()).into());
    }

    let mut cursor = Cursor::new(data);
    cursor.seek(std::io::SeekFrom::Start(QwkMessage::HEADER_SIZE as u64))?;
    let mut report = TossReport::default();
    let mut bases = OpenBases::default();
    while (cursor.position() as usize) < cursor.get_ref().len() {
        let message = QwkMessage::read(&mut cursor, true)?;
        let Some(area) = hub.areas.iter().find(|area| area.remote_conference == message.conference_number) else {
            report.unknown_conferences += 1;
            continue;
        };
        let metadata = Metadata::split(&message.text.to_string());
        if metadata.via.iter().any(|id| id.eq_ignore_ascii_case(&config.local_id)) {
            report.loops += 1;
            continue;
        }
        let base = bases.get(&get_path(board_root, &area.local_area))?;
        if let Some(id) = &metadata.msgid {
            let crc = JamMessageBase::crc(&BString::from(id.as_str()));
            if !base.seen.insert(crc) {
                report.duplicates += 1;
                continue;
            }
        }
        let written = message.date_time().and_utc();
        let mut jam = JamMessage::default()
            .with_from(message.from)
            .with_to(message.to)
            .with_subject(message.subj)
            .with_date_time(written)
            .with_attributes(attributes::MSG_TYPEECHO)
            .with_text(BString::from(to_base_text(&metadata.body)))
            .with_sub_field(MessageSubfield::new(SubfieldType::Address0, BString::from(route(hub, &metadata.via))));
        if let Some(id) = metadata.msgid {
            jam = jam.with_msg_id(BString::from(id));
        }
        if let Some(id) = metadata.reply {
            jam = jam.with_reply_id(BString::from(id));
        }
        base.base.write_message(&jam)?;
        base.base.write_jhr_header()?;
        report.imported += 1;
    }
    Ok(report)
}

fn validate_id(id: &str) -> Res<()> {
    let valid = (2..=8).contains(&id.len())
        && id.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        && !matches!(id.to_ascii_uppercase().as_str(), "SYSOP" | "NETMAIL");
    if valid {
        Ok(())
    } else {
        Err(format!("Invalid QWKnet ID {id:?}; use 2-8 DOS-safe characters beginning with a letter").into())
    }
}

fn load_state(path: &Path) -> Res<ScanState> {
    if !path.is_file() {
        return Ok(ScanState::default());
    }
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

fn save_state(path: &Path, state: &ScanState) -> Res<()> {
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, toml::to_string(state)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn write_rep(path: &Path, hub_id: &str, messages: &[QwkMessage]) -> Res<()> {
    let mut data = hub_id.as_bytes().to_vec();
    data.resize(QwkMessage::HEADER_SIZE, b' ');
    for message in messages {
        message.write(&mut data, true)?;
    }
    let temporary = path.with_extension("rep.tmp");
    let mut zip = zip::ZipWriter::new(fs::File::create(&temporary)?);
    zip.start_file(format!("{}.MSG", hub_id.to_ascii_uppercase()), SimpleFileOptions::default())?;
    zip.write_all(&data)?;
    zip.finish()?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn validate_qwk_archive(data: &[u8], hub_id: &str) -> Res<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data))?;
    if !(0..archive.len()).any(|index| archive.by_index(index).is_ok_and(|file| file.name().eq_ignore_ascii_case("messages.dat"))) {
        return Err(format!("QWKnet hub {hub_id} returned a packet without MESSAGES.DAT").into());
    }
    Ok(())
}

fn next_packet_name(inbound: &Path, hub_id: &str) -> PathBuf {
    let first = inbound.join(format!("{hub_id}.qwk"));
    if !first.exists() {
        return first;
    }
    for number in 0..=9999 {
        let candidate = inbound.join(format!("{hub_id}.{number:04}.qwk"));
        if !candidate.exists() {
            return candidate;
        }
    }
    inbound.join(format!("{}.{}.qwk", hub_id, Utc::now().timestamp()))
}

fn written_here(value: u32) -> bool {
    value & attributes::MSG_LOCAL != 0 || value & (attributes::MSG_TYPEECHO | attributes::MSG_TYPENET) == 0
}

/// QWK separates lines with 0xE3, which `jamjam` encodes from and decodes to
/// `\n`, while the message bases here keep the `\r` the rest of the board writes.
fn to_packet_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn to_base_text(text: &str) -> String {
    text.replace("\r\n", "\r").replace('\n', "\r")
}

/// Opening a base means reading the message ids it already holds, so a packet
/// that meets the same area message after message pays for it once.
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
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let base = if path.with_extension("jhr").is_file() {
                JamMessageBase::open(path)?
            } else {
                JamMessageBase::create(path)?
            };
            let seen = (base.lowest_message_number()..=base.highest_message_number())
                .filter_map(|number| base.read_header(number).ok())
                .map(|header| header.msgid_crc)
                .filter(|crc| *crc != NO_MSGID)
                .collect();
            self.bases.insert(path.to_path_buf(), OpenBase { base, seen });
        }
        Ok(self.bases.get_mut(path).expect("the base was just inserted"))
    }
}

fn subfield(header: &jamjam::jam::msg_header::JamMessageHeader, kind: SubfieldType) -> Option<String> {
    header
        .sub_fields
        .iter()
        .find(|field| field.field_type() == kind)
        .map(|field| field.content().to_string())
}

fn outbound_text(msgid: &str, reply: Option<&str>, body: &str) -> String {
    let mut text = format!("@MSGID: {msgid}\r");
    if let Some(reply) = reply {
        text.push_str("@REPLY: ");
        text.push_str(reply);
        text.push('\r');
    }
    text.push_str(body);
    text
}

#[derive(Default)]
struct Metadata {
    via: Vec<String>,
    msgid: Option<String>,
    reply: Option<String>,
    body: String,
}

impl Metadata {
    fn split(text: &str) -> Self {
        let mut result = Self::default();
        let mut lines = text.split_inclusive(['\r', '\n']).peekable();
        while let Some(line) = lines.peek().copied() {
            let clean = line.trim_end_matches(['\r', '\n']);
            let Some((name, value)) = clean.split_once(':') else { break };
            match name.to_ascii_uppercase().as_str() {
                "@VIA" => result.via = value.trim().split('/').map(str::to_string).collect(),
                "@MSGID" => result.msgid = Some(value.trim().to_string()),
                "@REPLY" => result.reply = Some(value.trim().to_string()),
                "@TZ" => {}
                _ => break,
            }
            lines.next();
        }
        result.body = lines.collect::<String>().trim_end_matches([' ', '\0']).to_string();
        result
    }
}

fn route(hub: &QwkHub, via: &[String]) -> String {
    std::iter::once(hub.id.as_str())
        .chain(via.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_removed_and_a_return_route_is_kept() {
        let metadata = Metadata::split("@VIA: MID/ORIGIN\r@MSGID: <one@origin>\rBody\r");
        assert_eq!(metadata.via, ["MID", "ORIGIN"]);
        assert_eq!(metadata.msgid.as_deref(), Some("<one@origin>"));
        assert_eq!(metadata.body, "Body\r");
        assert_eq!(
            route(
                &QwkHub {
                    id: "HUB".into(),
                    ..Default::default()
                },
                &metadata.via
            ),
            "HUB/MID/ORIGIN"
        );
    }

    #[test]
    fn qwk_ids_follow_the_network_contract() {
        assert!(validate_id("ICY_BBS").is_ok());
        assert!(validate_id("1BAD").is_err());
        assert!(validate_id("NETMAIL").is_err());
        assert!(validate_id("TOO-LONG-ID").is_err());
    }

    #[test]
    fn a_local_message_round_trips_through_rep_and_qwk() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        let mut base = JamMessageBase::create(&source).unwrap();
        base.write_message(
            &JamMessage::default()
                .with_from(BString::from("Alice"))
                .with_to(BString::from("All"))
                .with_subject(BString::from("Hello"))
                .with_attributes(attributes::MSG_LOCAL)
                .with_text(BString::from("DOVE body\r")),
        )
        .unwrap();
        base.write_jhr_header().unwrap();

        let mut config = QwkNetworkConfig {
            enabled: true,
            local_id: "ICYTEST".into(),
            inbound: "inbound".into(),
            outbound: "outbound".into(),
            hubs: vec![QwkHub {
                id: "VERT".into(),
                areas: vec![QwkHubArea {
                    remote_conference: 2001,
                    local_area: "source".into(),
                    read_only: false,
                }],
                ..Default::default()
            }],
        };
        let scanned = scan(&config, root.path(), "VERT").unwrap();
        assert_eq!(scanned.messages, 1);

        let rep = scanned.packet.unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(rep).unwrap()).unwrap();
        let mut messages = Vec::new();
        archive.by_name("VERT.MSG").unwrap().read_to_end(&mut messages).unwrap();
        let body = &messages[QwkMessage::HEADER_SIZE * 2..];
        assert!(body.contains(&0xE3), "the packet has to break lines the way QWK does");
        assert!(!body.contains(&b'\r'), "a raw carriage return would hide the kludges from the hub");

        let qwk = root.path().join("VERT.qwk");
        let mut packet = zip::ZipWriter::new(fs::File::create(&qwk).unwrap());
        packet.start_file("MESSAGES.DAT", SimpleFileOptions::default()).unwrap();
        packet.write_all(&messages).unwrap();
        packet.finish().unwrap();

        config.hubs[0].areas[0].local_area = target;
        let tossed = toss(&config, root.path(), "VERT", &qwk).unwrap();
        assert_eq!(tossed.imported, 1);
        let imported = JamMessageBase::open(root.path().join("target")).unwrap();
        let header = imported.read_header(imported.lowest_message_number()).unwrap();
        assert_eq!(header.subject().unwrap(), "Hello");
        assert_eq!(imported.read_message_text(&header).unwrap(), "DOVE body\r");

        let duplicate = toss(&config, root.path(), "VERT", &qwk).unwrap();
        assert_eq!(duplicate.duplicates, 1);

        fs::remove_file(root.path().join("outbound").join("VERT.rep")).unwrap();
        let rescan = scan(&config, root.path(), "VERT").unwrap();
        assert_eq!(rescan.messages, 0, "mail taken from the hub must not travel back to it");
    }

    #[test]
    fn a_renamed_area_keeps_its_scan_pointer() {
        let root = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(root.path().join("source")).unwrap();
        base.write_message(
            &JamMessage::default()
                .with_from(BString::from("Alice"))
                .with_subject(BString::from("Hello"))
                .with_attributes(attributes::MSG_LOCAL)
                .with_text(BString::from("Body\r")),
        )
        .unwrap();
        base.write_jhr_header().unwrap();

        let mut config = QwkNetworkConfig {
            enabled: true,
            local_id: "ICYTEST".into(),
            inbound: "inbound".into(),
            outbound: "outbound".into(),
            hubs: vec![QwkHub {
                id: "VERT".into(),
                areas: vec![QwkHubArea {
                    remote_conference: 2001,
                    local_area: "source".into(),
                    read_only: false,
                }],
                ..Default::default()
            }],
        };
        assert_eq!(scan(&config, root.path(), "VERT").unwrap().messages, 1);
        fs::remove_file(root.path().join("outbound").join("VERT.rep")).unwrap();

        for entry in fs::read_dir(root.path()).unwrap().filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(extension) = name.strip_prefix("source.") {
                fs::rename(entry.path(), root.path().join(format!("moved.{extension}"))).unwrap();
            }
        }
        config.hubs[0].areas[0].local_area = "moved".into();

        let rescan = scan(&config, root.path(), "VERT").unwrap();
        assert_eq!(rescan.messages, 0, "renaming an area must not send the whole base out again");
    }

    #[test]
    fn a_hub_conference_that_is_not_carried_here_is_counted_and_skipped() {
        let root = tempfile::tempdir().unwrap();
        let mut data = b"ICYTEST".to_vec();
        data.resize(QwkMessage::HEADER_SIZE, b' ');
        QwkMessage {
            status: b' ',
            msg_number: 2999,
            date_time: "01-02-26".to_string().into(),
            to: BString::from("All"),
            from: BString::from("Bob"),
            subj: BString::from("Stray"),
            password: BString::default(),
            ref_msg_number: 0,
            active_flag: MSG_ACTIVE,
            conference_number: 2999,
            logical_message_number: 1,
            net_tag: b'*',
            text: BString::from("Body\n"),
        }
        .write(&mut data, true)
        .unwrap();
        let qwk = root.path().join("VERT.qwk");
        let mut packet = zip::ZipWriter::new(fs::File::create(&qwk).unwrap());
        packet.start_file("MESSAGES.DAT", SimpleFileOptions::default()).unwrap();
        packet.write_all(&data).unwrap();
        packet.finish().unwrap();

        let config = QwkNetworkConfig {
            enabled: true,
            local_id: "ICYTEST".into(),
            inbound: "inbound".into(),
            outbound: "outbound".into(),
            hubs: vec![QwkHub {
                id: "VERT".into(),
                areas: vec![QwkHubArea {
                    remote_conference: 2001,
                    local_area: "general".into(),
                    read_only: false,
                }],
                ..Default::default()
            }],
        };
        let report = toss(&config, root.path(), "VERT", &qwk).unwrap();
        assert_eq!(report.unknown_conferences, 1);
        assert_eq!(report.imported, 0);
    }

    #[test]
    fn a_message_that_has_been_here_before_is_dropped() {
        let root = tempfile::tempdir().unwrap();
        let mut data = b"ICYTEST".to_vec();
        data.resize(QwkMessage::HEADER_SIZE, b' ');
        QwkMessage {
            status: b' ',
            msg_number: 2001,
            date_time: "01-02-26".to_string().into(),
            to: BString::from("All"),
            from: BString::from("Bob"),
            subj: BString::from("Looped"),
            password: BString::default(),
            ref_msg_number: 0,
            active_flag: MSG_ACTIVE,
            conference_number: 2001,
            logical_message_number: 1,
            net_tag: b'*',
            text: BString::from("@VIA: OTHER/ICYTEST\nBody\n"),
        }
        .write(&mut data, true)
        .unwrap();
        let qwk = root.path().join("VERT.qwk");
        let mut packet = zip::ZipWriter::new(fs::File::create(&qwk).unwrap());
        packet.start_file("MESSAGES.DAT", SimpleFileOptions::default()).unwrap();
        packet.write_all(&data).unwrap();
        packet.finish().unwrap();

        let config = QwkNetworkConfig {
            enabled: true,
            local_id: "ICYTEST".into(),
            inbound: "inbound".into(),
            outbound: "outbound".into(),
            hubs: vec![QwkHub {
                id: "VERT".into(),
                areas: vec![QwkHubArea {
                    remote_conference: 2001,
                    local_area: "general".into(),
                    read_only: false,
                }],
                ..Default::default()
            }],
        };
        let report = toss(&config, root.path(), "VERT", &qwk).unwrap();
        assert_eq!(report.loops, 1);
        assert_eq!(report.imported, 0);
    }
}
