use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use bstr::BString;
use chrono::{DateTime, NaiveDateTime, Utc};
use fs4::FileExt;
use jamjam::jam::{
    JamMessage, JamMessageBase, attributes,
    msg_header::{JamMessageHeader, MessageSubfield, SubfieldType},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

use super::{ZconnectConfig, ZconnectLink, board_name, fqdn, localpart};
use crate::{Res, icy_board::get_path};

// Deliberate resource limits, not wire-format limits. Oversized packets fail as a
// whole and remain on disk, rather than being truncated or partially imported.
const MAX_ARCHIVE: usize = 32 * 1024 * 1024;
const MAX_EXPANDED: usize = 32 * 1024 * 1024;
const MAX_BODY: usize = 512 * 1024;
const MAX_HEADER: usize = 32 * 1024;
const MAX_MESSAGES: usize = 4096;
const MAX_ENTRIES: usize = 128;
const MAX_STATE: usize = 4 * 1024 * 1024;

// Private JAM extension namespace. Header and body subfields preserve the exact
// wire bytes; repeated BODY fields concatenate in order. Do not re-export these.
const WIRE_HEADER: SubfieldType = SubfieldType::Unknown(0x5a43_0001);
const WIRE_BODY: SubfieldType = SubfieldType::Unknown(0x5a43_0002);

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
    /// Messages containing opaque content or at least one private recipient.
    pub unsupported: usize,
    /// Unmapped public recipients (each distinct EMP once per message).
    pub unknown_boards: usize,
}

/// Transport must validate the config and link before using this path. Unknown
/// identifiers never become path components, even though this helper is infallible.
pub fn pending_packet(config: &ZconnectConfig, root: &Path, id: &str) -> PathBuf {
    let key = config.link(id).filter(|l| super::spool_id(&l.id)).map_or("__invalid__", |l| l.id.as_str());
    get_path(root, &config.outbound).join(key).join("mail.zip")
}

fn checked_link<'a>(config: &'a ZconnectConfig, id: &str) -> Res<&'a ZconnectLink> {
    config.validate()?;
    if !config.enabled || !fqdn(&config.local_system) {
        return Err("ZCONNECT is disabled or lacks a valid local identity".into());
    }
    config.link(id).ok_or_else(|| format!("No ZCONNECT link named {id}").into())
}

/// The lock file is never unlinked: unlinking would allow two independent locks
/// on different inodes. File close releases the advisory lock on every exit path.
fn operation_lock(config: &ZconnectConfig, root: &Path, link: &ZconnectLink) -> Res<(PathBuf, File)> {
    let dir = get_path(root, &config.outbound).join(&link.id);
    fs::create_dir_all(&dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("operation.lock"))?;
    FileExt::lock(&file)?;
    Ok((dir, file))
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Journal {
    #[serde(default)]
    committed: BTreeMap<String, u32>,
    pending: Option<Pending>,
    /// An acknowledgement is durable BEFORE archive retirement. Recovery must
    /// finish retirement, not offer this already-acknowledged archive again.
    retired: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pending {
    next: BTreeMap<String, u32>,
    messages: usize,
    sha256: String,
}

fn read_limited(path: &Path, limit: usize) -> Res<Vec<u8>> {
    let file = File::open(path)?;
    if file.metadata()?.len() > limit as u64 {
        return Err(format!("ZCONNECT file exceeds size limit: {}", path.display()).into());
    }
    let mut data = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut data)?;
    if data.len() > limit {
        return Err("ZCONNECT file grew beyond size limit".into());
    }
    Ok(data)
}

fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn sync_directory(dir: &Path) -> Res<()> {
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Res<()> {
    let dir = path.parent().ok_or("ZCONNECT destination has no parent")?;
    fs::create_dir_all(dir)?;
    let mut temporary = tempfile::NamedTempFile::new_in(dir)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    sync_directory(dir)
}

fn load_journal(dir: &Path) -> Res<Journal> {
    let path = dir.join("scan.toml");
    if !path.exists() {
        return Ok(Journal::default());
    }
    Ok(toml::from_str(std::str::from_utf8(&read_limited(&path, MAX_STATE)?)?)?)
}

fn save_journal(dir: &Path, journal: &Journal) -> Res<()> {
    let text = toml::to_string(journal)?;
    if text.len() > MAX_STATE {
        return Err("ZCONNECT checkpoint exceeds size limit".into());
    }
    atomic_write(&dir.join("scan.toml"), text.as_bytes())
}

fn verify_archive(path: &Path, expected: &str) -> Res<()> {
    if digest(&read_limited(path, MAX_ARCHIVE)?) != expected {
        return Err(format!("ZCONNECT pending archive/checkpoint mismatch: {}", path.display()).into());
    }
    Ok(())
}

fn recover(dir: &Path, journal: &mut Journal) -> Res<()> {
    if journal.pending.is_some() && journal.retired.is_some() {
        return Err("Invalid ZCONNECT checkpoint: both pending and retired".into());
    }
    if let Some(hash) = &journal.retired {
        for name in ["mail.zip", "prepared.zip"] {
            let path = dir.join(name);
            if path.exists() {
                verify_archive(&path, hash)?;
                fs::remove_file(path)?;
            }
        }
        sync_directory(dir)?;
        journal.retired = None;
        save_journal(dir, journal)?;
    }
    if let Some(pending) = &journal.pending {
        let packet = dir.join("mail.zip");
        if !packet.exists() {
            let prepared = dir.join("prepared.zip");
            // Missing both files is an error. Never silently advance/rescan.
            verify_archive(&prepared, &pending.sha256)?;
            fs::rename(prepared, &packet)?;
            sync_directory(dir)?;
        }
        verify_archive(&packet, &pending.sha256)?;
    } else if dir.join("mail.zip").exists() {
        return Err("ZCONNECT mail.zip exists without a checkpoint; preserve and investigate it".into());
    }
    Ok(())
}

/// Prepare one immutable retryable archive. Only acknowledgement commits its
/// scan pointers. Calls are serialized per link; JAM reads hold a shared lock.
pub fn scan(config: &ZconnectConfig, board_root: &Path, link_id: &str) -> Res<ScanReport> {
    let link = checked_link(config, link_id)?;
    let (dir, _lock) = operation_lock(config, board_root, link)?;
    let mut journal = load_journal(&dir)?;
    recover(&dir, &mut journal)?;
    if let Some(pending) = &journal.pending {
        return Ok(ScanReport {
            messages: pending.messages,
            packet: Some(dir.join("mail.zip")),
        });
    }
    let mut next = journal.committed.clone();
    let mut data = Vec::new();
    let mut count = 0;
    for area in &link.areas {
        if area.read_only {
            continue;
        }
        let path = get_path(board_root, &area.local_area);
        if !path.with_extension("jhr").exists() {
            continue;
        }
        let mut base = JamMessageBase::open(&path)?;
        base.lock_shared()?;
        let key = area.remote_board.to_ascii_uppercase();
        let previous = next.get(&key).copied().unwrap_or(0);
        // Packing with renumbering is not safely distinguishable from replacement.
        // Fail closed instead of resetting the pointer and resending old mail.
        if base.highest_message_number() < previous {
            return Err(format!("ZCONNECT JAM numbering moved backwards for {}", area.remote_board).into());
        }
        let mut complete = true;
        for result in base.messages() {
            let header = result?;
            if header.message_number <= previous {
                continue;
            }
            if !exportable(&header) {
                next.insert(key.clone(), header.message_number);
                continue;
            }
            if header.attributes & (attributes::MSG_HOLD | attributes::MSG_LOCKED) != 0 {
                // Do not advance past a temporarily held local message.
                complete = false;
                break;
            }
            if header.txt_len as usize > MAX_BODY {
                return Err(format!("ZCONNECT message {} is too large", header.message_number).into());
            }
            let body = base.read_message_text(&header)?;
            let encoded = encode_local(config, &area.remote_board, &path, &base, &header, body.as_ref())?;
            if count == MAX_MESSAGES || data.len() + encoded.len() > MAX_EXPANDED {
                complete = false;
                break;
            }
            data.extend_from_slice(&encoded);
            count += 1;
            next.insert(key.clone(), header.message_number);
        }
        if complete {
            next.insert(key, base.highest_message_number());
        }
    }
    if count == 0 {
        // Only skipped records, never unsent eligible mail, can be committed here.
        journal.committed = next;
        save_journal(&dir, &journal)?;
        return Ok(ScanReport::default());
    }
    let archive = make_archive(&data)?;
    // Crash before the journal: prepared.zip is merely an orphan, safe to replace.
    // Crash after the journal: recover promotes that exact archive to mail.zip.
    atomic_write(&dir.join("prepared.zip"), &archive)?;
    journal.pending = Some(Pending {
        next,
        messages: count,
        sha256: digest(&archive),
    });
    save_journal(&dir, &journal)?;
    recover(&dir, &mut journal)?;
    Ok(ScanReport {
        messages: count,
        packet: Some(dir.join("mail.zip")),
    })
}

/// Called ONLY after verified remote success. Committing the checkpoint and the
/// retirement marker is one atomic write. Repeating an acknowledgement is safe.
pub fn acknowledge_outbound(config: &ZconnectConfig, board_root: &Path, link_id: &str) -> Res<()> {
    let link = checked_link(config, link_id)?;
    let (dir, _lock) = operation_lock(config, board_root, link)?;
    let mut journal = load_journal(&dir)?;
    recover(&dir, &mut journal)?;
    if let Some(pending) = journal.pending.take() {
        journal.committed = pending.next;
        journal.retired = Some(pending.sha256);
        save_journal(&dir, &journal)?;
        recover(&dir, &mut journal)?;
    }
    Ok(())
}

fn exportable(header: &JamMessageHeader) -> bool {
    let prohibited = attributes::MSG_PRIVATE
        | attributes::MSG_TYPENET
        | attributes::MSG_TYPELOCAL
        | attributes::MSG_DELETED
        | attributes::MSG_NODISP
        | attributes::MSG_FILEATTACH
        | attributes::MSG_FILEREQUEST
        | attributes::MSG_ENCRYPT
        | attributes::MSG_COMPRESS;
    header.attributes & prohibited == 0
        && !header.needs_password()
        && !header
            .sub_fields
            .iter()
            .any(|f| f.field_type() == WIRE_HEADER || f.field_type() == SubfieldType::Address0)
        && (header.attributes & attributes::MSG_LOCAL != 0 || header.attributes & attributes::MSG_TYPEECHO == 0)
}

fn field(header: &JamMessageHeader, kind: SubfieldType) -> Option<&[u8]> {
    header.sub_fields.iter().find(|f| f.field_type() == kind).map(|f| f.content().as_ref())
}

fn ascii_header(value: &str) -> Res<()> {
    if !value.bytes().all(|b| (32..=126).contains(&b)) {
        return Err("ZCONNECT headers require printable ASCII; cannot encode without losing data".into());
    }
    Ok(())
}

fn valid_mid(s: &str) -> bool {
    let Some((user, host)) = s.split_once('@') else {
        return false;
    };
    !user.is_empty() && user.len() <= 512 && user.bytes().all(|b| (33..=126).contains(&b) && !b"<>/@()[]\\\"".contains(&b)) && fqdn(host)
}

/// Use the same identity namespace for MID and BEZ. Never trim, case-fold or
/// lossily decode foreign IDs: hash their original bytes, including delimiters
/// when they are not a single RFC enclosure around an otherwise valid MID.
fn mapped_id(config: &ZconnectConfig, raw: &[u8]) -> String {
    if let Ok(id) = std::str::from_utf8(raw) {
        if valid_mid(id) {
            return id.to_string();
        }
        if let Some(inner) = id.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
            if valid_mid(inner) {
                return inner.to_string();
            }
        }
    }
    format!("jam-sha256-{}@{}", digest(raw), config.local_system)
}

fn local_message_id(config: &ZconnectConfig, path: &Path, header: &JamMessageHeader) -> String {
    if let Some(raw) = field(header, SubfieldType::MsgID) {
        return mapped_id(config, raw);
    }
    // Keep the existing fallback unchanged, shared by MID and native replies.
    // Stable across retries and links; full hash rather than a path CRC.
    let key = format!("{}\0{}\0{}", path.display(), header.message_number, header.date_written);
    format!("{}@{}", digest(key.as_bytes()), config.local_system)
}

fn encode_local(config: &ZconnectConfig, board: &str, path: &Path, base: &JamMessageBase, header: &JamMessageHeader, bytes: &[u8]) -> Res<Vec<u8>> {
    // Native IcyBoard JAM text is UTF-8. Never use BString's lossy Display here.
    let text = std::str::from_utf8(bytes)?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
    let mut body = Vec::with_capacity(normalized.len());
    for c in normalized.chars() {
        let n = c as u32;
        if n > 255 || !(matches!(n, 9 | 10 | 13) || (32..=126).contains(&n) || (160..=255).contains(&n)) {
            return Err("ZCONNECT local message cannot be represented losslessly as ISO1 public text".into());
        }
        body.push(n as u8);
    }
    if body.len() > MAX_BODY {
        return Err("ZCONNECT encoded body exceeds limit".into());
    }
    let from = std::str::from_utf8(header.from().map_or(&[], |s| s.as_ref()))?;
    let subject = std::str::from_utf8(header.subject().map_or(&[], |s| s.as_ref()))?;
    ascii_header(from)?;
    ascii_header(subject)?;
    let user = if localpart(from) { from } else { &config.local_user };
    let mut sender = format!("{user}@{}", config.local_system);
    if !from.is_empty() && !from.contains(['(', ')']) {
        sender.push_str(&format!(" ({from})"));
    } else if from.contains(['(', ')']) {
        return Err("ZCONNECT sender realname cannot contain parentheses".into());
    }
    let id = local_message_id(config, path, header);
    let date = DateTime::<Utc>::from_timestamp(i64::from(header.date_written), 0).ok_or("Invalid JAM date")?;
    let mut result = format!(
        "ABS: {sender}\r\nBET: {subject}\r\nMID: {id}\r\nEDA: {}W+0\r\nEMP: {board}\r\nROT: {}\r\nCHARSET: ISO1\r\n",
        date.format("%Y%m%d%H%M%S"),
        config.local_system
    );
    // An explicit ReplyID (including an imported ZCONNECT BEZ) is authoritative.
    let reply = field(header, SubfieldType::ReplyID).map(|raw| mapped_id(config, raw)).or_else(|| {
        if header.reply_to == 0 {
            return None;
        }
        // Resolve under scan's shared JAM lock. Missing/deleted parents are
        // optional references; never synthesize identities for private or
        // otherwise non-exportable parents from their local message numbers.
        let parent = base.read_header(header.reply_to).ok()?;
        exportable(&parent).then(|| local_message_id(config, path, &parent))
    });
    if let Some(reply) = reply {
        result.push_str(&format!("BEZ: {reply}\r\n"));
    }
    result.push_str(&format!("LEN: {}\r\n\r\n", body.len()));
    if result.len() > MAX_HEADER {
        return Err("ZCONNECT encoded header exceeds limit".into());
    }
    let mut result = result.into_bytes();
    result.extend_from_slice(&body);
    Ok(result)
}

fn make_archive(data: &[u8]) -> Res<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("MAIL.BRT", SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated))?;
    zip.write_all(data)?;
    let result = zip.finish()?.into_inner();
    if result.len() > MAX_ARCHIVE {
        return Err("ZCONNECT archive exceeds size limit".into());
    }
    Ok(result)
}

#[derive(Debug)]
struct WireMessage {
    fields: Vec<(String, Vec<u8>)>,
    header: Vec<u8>,
    body: Vec<u8>,
    date: DateTime<Utc>,
}

impl WireMessage {
    fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.fields.iter().filter(move |(key, _)| key == name).map(|(_, value)| value.as_slice())
    }
    fn value(&self, name: &str) -> Option<&[u8]> {
        self.fields.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_slice())
    }
    fn text(&self, name: &str) -> Res<&str> {
        Ok(std::str::from_utf8(self.value(name).ok_or_else(|| format!("Missing ZCONNECT {name}"))?)?)
    }
    fn has(&self, name: &str) -> bool {
        self.value(name).is_some()
    }
}

fn decimal(bytes: &[u8]) -> Res<usize> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return Err("ZCONNECT length is not an unsigned decimal".into());
    }
    Ok(std::str::from_utf8(bytes)?.parse()?)
}

/// EDA's numeric date is already GMT, even when it says S+2 or W-9:30.
fn parse_date(value: &str) -> Res<DateTime<Utc>> {
    if value.len() < 17
        || !value.is_ascii()
        || !value.as_bytes()[..14].iter().all(u8::is_ascii_digit)
        || !matches!(value.as_bytes()[14], b'S' | b'W')
        || !matches!(value.as_bytes()[15], b'+' | b'-')
    {
        return Err("Malformed ZCONNECT EDA".into());
    }
    let offset = &value[16..];
    let (hours, minutes) = offset.split_once(':').unwrap_or((offset, "0"));
    if decimal(hours.as_bytes())? > 23 || decimal(minutes.as_bytes())? > 59 || (offset.contains(':') && minutes.len() != 2) || hours.len() > 2 {
        return Err("Malformed ZCONNECT EDA timezone".into());
    }
    let date = NaiveDateTime::parse_from_str(&value[..14], "%Y%m%d%H%M%S")?.and_utc();
    if !(0..=u32::MAX as i64).contains(&date.timestamp()) || date.timestamp_subsec_nanos() != 0 {
        return Err("ZCONNECT date cannot be represented in JAM".into());
    }
    Ok(date)
}

fn parse_messages(data: &[u8], messages: &mut Vec<WireMessage>) -> Res<()> {
    let mut pos = 0;
    while pos < data.len() {
        if messages.len() >= MAX_MESSAGES {
            return Err("Too many ZCONNECT messages".into());
        }
        let remaining = &data[pos..];
        let end = remaining[..remaining.len().min(MAX_HEADER)]
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or("Missing or oversized ZCONNECT header terminator")?;
        for (i, b) in remaining[..end].iter().enumerate() {
            if (*b == b'\n' && (i == 0 || remaining[i - 1] != b'\r')) || (*b == b'\r' && remaining.get(i + 1) != Some(&b'\n')) {
                return Err("ZCONNECT headers require CRLF line endings".into());
            }
        }
        let mut fields = Vec::new();
        let mut singletons = HashSet::new();
        for line in remaining[..end].split(|b| *b == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let colon = line.iter().position(|b| *b == b':').ok_or("Malformed ZCONNECT header line")?;
            let key = &line[..colon];
            if key.is_empty() || key.len() > 100 || !key.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-') {
                return Err("Invalid ZCONNECT header name".into());
            }
            let key = std::str::from_utf8(key)?.to_ascii_uppercase();
            let value = &line[colon + 1..];
            let start = value.iter().position(|b| !matches!(*b, b' ' | b'\t')).unwrap_or(value.len());
            let value = &value[start..];
            if value.iter().any(|b| *b < 32 || *b == 127) {
                return Err("Control character in ZCONNECT header".into());
            }
            if is_singleton(&key) && !singletons.insert(key.clone()) {
                return Err(format!("Duplicate ZCONNECT {key} header").into());
            }
            fields.push((key, value.to_vec()));
        }
        let len = fields.iter().find(|(key, _)| key == "LEN").ok_or("Missing ZCONNECT LEN")?;
        let len = decimal(&len.1)?;
        if len > MAX_BODY {
            return Err("ZCONNECT body exceeds limit".into());
        }
        let body_start = pos + end + 4;
        let body_end = body_start
            .checked_add(len)
            .filter(|end| *end <= data.len())
            .ok_or("Truncated ZCONNECT LEN body")?;
        let mut message = WireMessage {
            fields,
            header: data[pos..body_start].to_vec(),
            body: data[body_start..body_end].to_vec(),
            date: DateTime::UNIX_EPOCH,
        };
        for required in ["ABS", "BET", "MID", "EDA", "EMP", "ROT"] {
            if !message.has(required) {
                return Err(format!("Missing ZCONNECT {required}").into());
            }
        }
        if !valid_mid(message.text("MID")?) {
            return Err("Invalid ZCONNECT MID".into());
        }
        message.date = parse_date(message.text("EDA")?)?;
        if !message.text("ROT")?.split('!').all(fqdn) {
            return Err("Invalid ZCONNECT ROT route".into());
        }
        let sender = message.text("ABS")?;
        let addr = sender.split_once(' ').map_or(sender, |(addr, _)| addr);
        if !valid_mid(addr) {
            return Err("Invalid ZCONNECT ABS address".into());
        }
        for reply in message.values("BEZ") {
            if !valid_mid(std::str::from_utf8(reply)?) {
                return Err("Invalid ZCONNECT BEZ".into());
            }
        }
        if let Some(comment) = message.value("KOM") {
            if decimal(comment)? > len {
                return Err("ZCONNECT KOM exceeds LEN".into());
            }
        }
        messages.push(message);
        pos = body_end;
    }
    Ok(())
}

fn is_singleton(key: &str) -> bool {
    matches!(
        key,
        "ABS"
            | "BET"
            | "MID"
            | "EDA"
            | "ROT"
            | "LEN"
            | "CHARSET"
            | "TYP"
            | "KOM"
            | "CRYPT"
            | "CRYPT-CONTENT-TYP"
            | "ERR"
            | "FILE"
            | "LANGUAGE"
            | "LIFETIME"
            | "MAILER"
            | "O-ROT"
            | "O-EDA"
            | "OAB"
            | "OEM"
            | "ORG"
            | "PRIO"
            | "SIGNED"
            | "PGP-SIG"
            | "PGP-PUBLIC-KEY"
            | "WAB"
            | "ERSETZT"
    )
}

fn parse_archive(bytes: &[u8]) -> Res<Vec<WireMessage>> {
    if bytes.len() > MAX_ARCHIVE {
        return Err("ZCONNECT archive exceeds limit".into());
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    if zip.len() > MAX_ENTRIES {
        return Err("Too many ZIP entries".into());
    }
    let mut expanded = 0;
    let mut names = HashSet::new();
    let mut messages = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.enclosed_name().is_none()
            || entry.name().starts_with('/')
            || entry.name().contains(['\\', ':'])
            || entry.name().split('/').any(|s| s == ".." || s == ".")
            || !names.insert(entry.name().to_ascii_lowercase())
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("Unsafe or duplicate ZCONNECT ZIP member".into());
        }
        if entry.is_dir() {
            continue;
        }
        let available = MAX_EXPANDED - expanded;
        if entry.size() > available as u64 {
            return Err("Expanded ZIP size exceeds limit".into());
        }
        let mut data = Vec::new();
        (&mut entry).take(available as u64 + 1).read_to_end(&mut data)?;
        if data.len() > available || data.len() as u64 != entry.size() {
            return Err("Invalid expanded ZIP size".into());
        }
        expanded += data.len();
        // Every member is parsed, irrespective of its extension or advertised type.
        parse_messages(&data, &mut messages)?;
    }
    Ok(messages)
}

fn decode_body(message: &WireMessage) -> Option<String> {
    if ["TYP", "CRYPT", "ERR", "ERSETZT"].iter().any(|name| message.has(name))
        || message.value("KOM").is_some_and(|v| v != b"0")
        || message.fields.iter().any(|(_, value)| !value.is_ascii())
    {
        return None;
    }
    let iso1 = match message.value("CHARSET") {
        Some(s) if s.eq_ignore_ascii_case(b"ISO1") => true,
        None => false,
        Some(s) if s.eq_ignore_ascii_case(b"ZCONNECT3.0") => false,
        _ => return None,
    };
    let mut out = String::new();
    let mut pos = 0;
    while pos < message.body.len() {
        let b = message.body[pos];
        match b {
            b'\r' => {
                if message.body.get(pos + 1) != Some(&b'\n') {
                    return None;
                }
                out.push('\r');
                pos += 2;
                continue;
            }
            b'\t' | 32..=126 => out.push(char::from(b)),
            160..=255 if iso1 => out.push(char::from(b)),
            // The supplied 3.1 spec references, but does not reproduce, the
            // 3.0 extended-character mapping. Its unambiguous ASCII subset is
            // supported; do NOT guess CP437/CP850 for the remaining bytes.
            _ => return None,
        }
        pos += 1;
    }
    Some(out)
}

struct Delivery {
    message: usize,
    board: String,
}

/// Validate and classify the complete archive before opening any JAM base for
/// writing. The original input is NEVER removed, including on successful import.
/// Unsupported/unknown-recipient archives also get a durable content-addressed
/// retention copy before any imports, so caller cleanup cannot lose that content.
pub fn toss(config: &ZconnectConfig, board_root: &Path, link_id: &str, packet: &Path) -> Res<TossReport> {
    let link = checked_link(config, link_id)?;
    let (_dir, _lock) = operation_lock(config, board_root, link)?;
    let bytes = read_limited(packet, MAX_ARCHIVE)?;
    let messages = parse_archive(&bytes)?;
    let mut report = TossReport::default();
    let decoded: Vec<_> = messages.iter().map(decode_body).collect();
    let mut delivery_count = 0;
    let mut deliveries: BTreeMap<PathBuf, Vec<Delivery>> = BTreeMap::new();
    for (index, message) in messages.iter().enumerate() {
        let body = &decoded[index];
        let mut unsupported = body.is_none();
        let mut boards = HashSet::new();
        let looped = message.text("ROT")?.split('!').any(|host| host.eq_ignore_ascii_case(&config.local_system));
        if looped {
            report.loops += 1;
        }
        for raw in message.values("EMP") {
            let Ok(board) = std::str::from_utf8(raw) else {
                unsupported = true;
                continue;
            };
            if !board_name(board) {
                // Includes /BOARD@host: a PRIVATE recipient even if it names a board.
                unsupported = true;
                continue;
            }
            if !boards.insert(board.to_string()) {
                continue;
            }
            let Some(area) = link.areas.iter().find(|area| area.remote_board == board) else {
                report.unknown_boards += 1;
                continue;
            };
            if !looped && body.is_some() {
                delivery_count += 1;
                if delivery_count > MAX_MESSAGES {
                    return Err("Too many ZCONNECT crosspost deliveries".into());
                }
                deliveries.entry(get_path(board_root, &area.local_area)).or_default().push(Delivery {
                    message: index,
                    board: board.to_string(),
                });
            }
        }
        if unsupported {
            report.unsupported += 1;
        }
    }
    if report.unsupported > 0 || report.unknown_boards > 0 {
        let retained = get_path(board_root, &config.inbound)
            .join(&link.id)
            .join("retained")
            .join(format!("{}.zip", digest(&bytes)));
        atomic_write(&retained, &bytes)?;
    }
    for (path, batch) in deliveries {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut base = if path.with_extension("jhr").exists() {
            JamMessageBase::open(&path)?
        } else {
            JamMessageBase::create(&path)?
        };
        base.lock()?;
        let mut seen = HashSet::<Vec<u8>>::new();
        for result in base.messages() {
            let header = result?;
            if let Some(id) = field(&header, SubfieldType::MsgID) {
                seen.insert(id.to_vec());
            }
        }
        for delivery in batch {
            let message = &messages[delivery.message];
            let id = message.text("MID")?;
            if seen.contains(id.as_bytes()) {
                report.duplicates += 1;
                continue;
            }
            let mut jam = JamMessage::default()
                .with_from(BString::from(message.text("ABS")?))
                .with_to(BString::from(delivery.board))
                .with_subject(BString::from(message.text("BET")?))
                .with_date_time(message.date)
                .with_attributes(attributes::MSG_TYPEECHO)
                .with_msg_id(BString::from(id))
                .with_text(BString::from(decoded[delivery.message].as_deref().ok_or("Missing decoded ZCONNECT body")?))
                .with_sub_field(MessageSubfield::new(
                    SubfieldType::Address0,
                    BString::from(format!("{}!{}", config.local_system, message.text("ROT")?)),
                ))
                .with_sub_field(MessageSubfield::new(WIRE_HEADER, BString::from(message.header.clone())));
            if let Some(reply) = message.values("BEZ").last() {
                jam = jam.with_reply_id(BString::from(reply.to_vec()));
            }
            for chunk in message.body.chunks(60 * 1024) {
                jam = jam.with_sub_field(MessageSubfield::new(WIRE_BODY, BString::from(chunk.to_vec())));
            }
            base.write_message(&jam)?;
            base.write_jhr_header()?;
            // Make each imported message durable before counting it. Partial I/O
            // failures may leave a prefix; retry uses full IDs, per destination base.
            base.sync()?;
            seen.insert(id.as_bytes().to_vec());
            report.imported += 1;
        }
    }
    Ok(report)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
