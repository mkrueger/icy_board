//! Reader-scoped capture, not the standalone QWK area's independent selection.
//!
//! MSGREAD.C readmessage() skips group-password mail during capture (even when
//! the caller previously entered its password). C asks before download, D sends
//! text, Z compresses text, and QWK produces an indexed packet. Unlike the DOS
//! implementation, read effects are deferred until the protocol confirms delivery.
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{self, Write},
    path::{Path, PathBuf},
};

use bstr::BString;
use codepages::tables::UNICODE_TO_CP437;
use jamjam::{
    jam::{
        JamMessageBase,
        msg_header::{JamMessageHeader, SubfieldType},
    },
    qwk::{
        control::{Conference, ControlDat},
        qwk_message::QwkMessage,
    },
    util::basic_real::BasicReal,
};
use tempfile::TempDir;
use tokio::time::{Duration, Instant, timeout, timeout_at};
use zip::write::SimpleFileOptions;

use super::{
    HeaderLength, IceText, IcyBoardState, MessageFilter, MessageViewer, MsgFunc, ReadCommand, ReaderExit, ReaderOptions, Res, TerminalTarget,
    advance_read_pointer, attributes, display_flags, may_read_header, next_in_range, record_recipient_read, requires_read_password,
};
use crate::icy_board::{
    limits::{self, BatchSoFar, TransferHistory},
    state::user_commands::pcb::u_upload_file::create_protocol,
};

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_MESSAGES: usize = 10_000;
const MAX_SCAN: usize = 100_000;

pub(super) fn requested(cmd: &ReadCommand) -> bool {
    cmd.open_capture || cmd.open_qwk
}

pub(super) fn unsupported(cmd: &ReadCommand) -> Option<&'static str> {
    if cmd.net {
        // ConferenceFlags::NetStatus exists, but reader NET also requires the
        // original echoed/to-you selection, routing/tag lines, loop prevention,
        // NETFLAGS.DAT and a stable network conference map. The standalone
        // qwknet hub exporter does not accept reader ranges or caller identity.
        Some("Reader NET capture is unavailable: per-user network routing and NETFLAGS export are not implemented.")
    } else if cmd.cap_bye && !requested(cmd) {
        Some("BYE/GB requires a C, D, Z or QWK capture command.")
    } else if requested(cmd) && cmd.quick_scan {
        Some("Capture cannot be combined with quick scan.")
    } else if requested(cmd) && !matches!(cmd.func, MsgFunc::None | MsgFunc::Redisplay) {
        Some("Capture cannot be combined with a message action.")
    } else {
        None
    }
}

fn failure(message: &'static str) -> io::Error {
    io::Error::other(message)
}

fn message_limit(system: u16, user: u16) -> usize {
    usize::from(if user == 0 { system } else { system.min(user) })
}

/// Control/header fields cannot inject extra records or terminal commands.
fn field(text: &str) -> BString {
    text.chars()
        .filter(|ch| !ch.is_control())
        .take(255)
        .map(|ch| UNICODE_TO_CP437.get(&ch).copied().unwrap_or(b'?'))
        .collect::<Vec<_>>()
        .into()
}

fn body_bytes(text: &str) -> BString {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .map(|ch| match ch {
            '\n' => b'\n',
            '\t' => b'\t',
            _ => UNICODE_TO_CP437.get(&ch).copied().unwrap_or(b'?'),
        })
        .collect::<Vec<_>>()
        .into()
}

/// A separate permission check remains mandatory even with a caller-supplied
/// MessageFilter::default(), whose historical default grants read-all access.
fn exportable(header: &JamMessageHeader, user: &str, alias: &str, read_all: bool) -> bool {
    !header.is_deleted() && may_read_header(header, user, alias, read_all) && !requires_read_password(header, read_all)
}

struct ReadEffect {
    path: PathBuf,
    conference: u16,
    area: usize,
    header: JamMessageHeader,
}

pub(super) struct ReaderCapture {
    qwk: bool,
    zipped: bool,
    ask: bool,
    bye: bool,
    data: Vec<u8>,
    control: ControlDat,
    numbers: HashMap<(u16, usize), u16>,
    indexes: BTreeMap<u16, Vec<u8>>,
    personal: Vec<u8>,
    effects: Vec<ReadEffect>,
    seen: HashSet<(PathBuf, u32)>,
    per_conf: HashMap<u16, usize>,
    max_messages: usize,
    max_per_conf: usize,
    memory: usize,
    scanned: usize,
    limited: bool,
    attachments: bool,
}

impl ReaderCapture {
    fn new(cmd: &ReadCommand, control: ControlDat, numbers: HashMap<(u16, usize), u16>, max_messages: usize, max_per_conf: usize) -> Self {
        let mut data = Vec::new();
        if cmd.open_qwk {
            data.extend_from_slice(b"Produced by Icy Board");
            data.resize(128, b' ');
        }
        Self {
            qwk: cmd.open_qwk,
            zipped: cmd.zip_cap,
            ask: cmd.cap_ask,
            bye: cmd.cap_bye,
            data,
            control,
            numbers,
            indexes: BTreeMap::new(),
            personal: Vec::new(),
            effects: Vec::new(),
            seen: HashSet::new(),
            per_conf: HashMap::new(),
            max_messages: max_messages.min(MAX_MESSAGES),
            max_per_conf,
            memory: 128,
            scanned: 0,
            limited: false,
            attachments: false,
        }
    }

    fn full(&mut self, conference: u16) -> bool {
        let full = self.effects.len() >= self.max_messages || self.per_conf.get(&conference).copied().unwrap_or(0) >= self.max_per_conf;
        self.limited |= full;
        full
    }

    fn append(
        &mut self,
        effect: ReadEffect,
        text: &str,
        viewer: &MessageViewer,
        length: HeaderLength,
        conf_name: &str,
        area_name: &str,
        personal: bool,
    ) -> Res<()> {
        let header = &effect.header;
        let key = (effect.path.clone(), header.message_number);
        if self.seen.contains(&key) {
            return Ok(());
        }
        if self.full(effect.conference) {
            return Ok(());
        }
        let header_bytes: usize = header.sub_fields.iter().map(|field| field.content().len() + 16).sum();
        if text.len() > MAX_MESSAGE_BYTES || header_bytes > MAX_MESSAGE_BYTES {
            return Err(failure("Capture message exceeds the safety size limit").into());
        }
        let mut encoded = Vec::new();
        let mut index = None;
        if self.qwk {
            let number = *self
                .numbers
                .get(&(effect.conference, effect.area))
                .ok_or_else(|| failure("No unambiguous QWK conference number"))?;
            let date = chrono::DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_default();
            let message = QwkMessage {
                status: match (header.is_private() || header.needs_password(), header.is_read()) {
                    (true, true) => b'+',
                    (true, false) => b'*',
                    (false, true) => b'-',
                    _ => b' ',
                },
                msg_number: header.message_number,
                date_time: date.format("%m-%d-%y%H:%M").to_string().into(),
                to: field(&header.to().map(ToString::to_string).unwrap_or_default()),
                from: field(&header.from().map(ToString::to_string).unwrap_or_default()),
                subj: field(&header.subject().map(ToString::to_string).unwrap_or_default()),
                // Never export the password/hash, JAM kludges or host paths.
                password: BString::default(),
                ref_msg_number: header.reply_to,
                active_flag: 225,
                conference_number: number,
                logical_message_number: self.effects.len() as u16 + 1,
                net_tag: b' ',
                text: body_bytes(text),
            };
            message.write(&mut encoded, true)?;
            let mut entry = BasicReal::from((self.data.len() / 128 + 1) as i32).bytes().to_vec();
            entry.push(number.min(255) as u8);
            index = Some((number, entry));
        } else {
            let date = chrono::DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_default();
            let lines = [
                viewer.format_hdr_text(&viewer.date_num.text, &date.to_string(), &header.message_number.to_string()),
                viewer.format_hdr_text(&viewer.to_line.text, &header.to().map(ToString::to_string).unwrap_or_default(), ""),
                viewer.format_hdr_text(&viewer.from_line.text, &header.from().map(ToString::to_string).unwrap_or_default(), ""),
                viewer.format_hdr_text(&viewer.subj_line.text, &header.subject().map(ToString::to_string).unwrap_or_default(), ""),
            ];
            for line in lines {
                encoded.extend_from_slice(&field(&line));
                encoded.extend_from_slice(b"\r\n");
            }
            if length == HeaderLength::Long {
                let status = if header.is_private() { &viewer._rcv_only.text } else { &viewer._public.text };
                let line = viewer.format_hdr_text(&viewer._read.text, if header.is_read() { "Read" } else { &viewer._not_read.text }, status);
                encoded.extend_from_slice(&field(&line));
                encoded.extend_from_slice(b"\r\n");
                encoded.extend_from_slice(&field(&viewer.format_hdr_text(&viewer.confarea.text, conf_name, area_name)));
                encoded.extend_from_slice(b"\r\n");
            }
            for byte in body_bytes(text).iter() {
                if *byte == b'\n' {
                    encoded.push(b'\r');
                }
                encoded.push(*byte);
            }
            encoded.extend_from_slice(b"\r\n<<<>>>\r\n");
        }
        let cost = encoded
            .len()
            .saturating_add(header_bytes)
            .saturating_add(effect.path.as_os_str().len())
            .saturating_add(512);
        if self.memory.saturating_add(cost) > MAX_BYTES {
            return Err(failure("Capture exceeds the 16 MiB safety limit").into());
        }
        self.memory += cost;
        if let Some((number, entry)) = index {
            if !self.indexes.contains_key(&number) {
                self.control.conferences.push(Conference {
                    number,
                    name: field(area_name),
                });
            }
            self.indexes.entry(number).or_default().extend_from_slice(&entry);
            if personal {
                self.personal.extend_from_slice(&entry);
            }
        }
        self.attachments |= header.attributes & attributes::MSG_FILEATTACH != 0
            || header
                .sub_fields
                .iter()
                .any(|field| matches!(field.field_type(), SubfieldType::EnclFile | SubfieldType::EnclFwAlias));
        self.data.extend_from_slice(&encoded);
        self.control.message_count += 1;
        *self.per_conf.entry(effect.conference).or_default() += 1;
        self.seen.insert(key);
        self.effects.push(effect);
        Ok(())
    }

    fn packet(&self) -> Res<CaptureFile> {
        let directory = tempfile::tempdir()?;
        // Neither BBSID, message fields nor user input ever supplies a path.
        let path = directory.path().join(if self.qwk {
            "mail.qwk"
        } else if self.zipped {
            "messages.zip"
        } else {
            "messages.txt"
        });
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        if self.qwk || self.zipped {
            let mut zip = zip::ZipWriter::new(file);
            let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            if self.qwk {
                zip.start_file("CONTROL.DAT", options)?;
                zip.write_all(&self.control.to_vec())?;
                zip.start_file("MESSAGES.DAT", options)?;
                zip.write_all(&self.data)?;
                for (number, index) in &self.indexes {
                    zip.start_file(format!("{number:03}.NDX"), options)?;
                    zip.write_all(index)?;
                }
                zip.start_file("PERSONAL.NDX", options)?;
                zip.write_all(&self.personal)?;
            } else {
                zip.start_file("MESSAGES.TXT", options)?;
                zip.write_all(&self.data)?;
            }
            file = zip.finish()?;
        } else {
            file.write_all(&self.data)?;
        }
        file.sync_all()?;
        if file.metadata()?.len() > MAX_BYTES as u64 {
            return Err(failure("Capture archive exceeds the safety limit").into());
        }
        drop(file);
        Ok(CaptureFile { _directory: directory, path })
    }
}

/// The owner remains on the async stack through transfer, errors and cancellation.
struct CaptureFile {
    _directory: TempDir,
    path: PathBuf,
}

impl IcyBoardState {
    async fn make_reader_capture(&self, cmd: &ReadCommand) -> Res<ReaderCapture> {
        let board = self.get_board().await;
        let config = &board.config.qwk_settings;
        let user = self.session.current_user.as_ref().ok_or_else(|| failure("Capture requires a logged-in user"))?;
        let limits = user.qwk_config.clone().unwrap_or_default();
        let choose = |value: &str, fallback: &str| field(if value.is_empty() { fallback } else { value });
        let control = ControlDat {
            bbs_name: choose(&config.bbs_name, &board.config.board.name),
            bbs_city_and_state: choose(&config.bbs_city_and_state, &board.config.board.location),
            bbs_phone_number: choose(&config.bbs_phone_number, &board.config.board.notice),
            bbs_sysop_name: choose(&config.bbs_sysop_name, &board.config.board.operator),
            bbs_id: choose(&config.bbs_id, &board.config.board.name),
            serial_number: 0,
            creation_time: chrono::Local::now().format("%m/%d/%y,%H:%M").to_string().into(),
            qmail_user_name: field(&self.session.user_name),
            qmail_menu_name: BString::default(),
            zero_line: "0".into(),
            message_count: 0,
            conferences: Vec::new(),
            welcome_screen: BString::default(),
            news_screen: BString::default(),
            logoff_screen: BString::default(),
            extra_lines: Vec::new(),
        };
        let mut numbers = HashMap::new();
        if cmd.open_qwk {
            let mut used = HashSet::new();
            // Reserve explicit IDs first, then assign zero-configured areas in
            // board order. Duplicate explicit IDs fail rather than misroute REP.
            for (conf, conference) in board.conferences.iter().enumerate() {
                if let Some(areas) = &conference.areas {
                    for (area, data) in areas.iter().enumerate() {
                        let conf = u16::try_from(conf).map_err(|_| failure("Too many QWK conferences"))?;
                        if data.qwk_conference_number != 0 {
                            if !used.insert(data.qwk_conference_number) {
                                return Err(failure("Duplicate configured QWK conference number").into());
                            }
                            numbers.insert((conf, area), data.qwk_conference_number);
                        }
                    }
                }
            }
            let mut next = 1u32;
            for (conf, conference) in board.conferences.iter().enumerate() {
                if let Some(areas) = &conference.areas {
                    for (area, data) in areas.iter().enumerate() {
                        if data.qwk_conference_number == 0 {
                            while next <= u16::MAX as u32 && used.contains(&(next as u16)) {
                                next += 1;
                            }
                            if next > u16::MAX as u32 {
                                return Err(failure("Too many QWK areas").into());
                            }
                            numbers.insert((u16::try_from(conf).map_err(|_| failure("Too many QWK conferences"))?, area), next as u16);
                            used.insert(next as u16);
                        }
                    }
                }
            }
        }
        Ok(ReaderCapture::new(
            cmd,
            control,
            numbers,
            message_limit(config.max_msgs, limits.max_msgs),
            message_limit(config.max_msgs_per_conf, limits.max_msgs_per_conf),
        ))
    }

    pub(super) async fn run_reader_capture(
        &mut self,
        base: &mut JamMessageBase,
        viewer: &MessageViewer,
        cmd: ReadCommand,
        options: &mut ReaderOptions,
        filter: Option<MessageFilter>,
    ) -> Res<()> {
        let search = self.session.search_pattern.clone();
        let result: Res<()> = async {
            options.capture = Some(self.make_reader_capture(&cmd).await?);
            if cmd.all_conf {
                if !matches!(self.read_all_conferences(viewer, &cmd, options).await?, ReaderExit::Done) {
                    return Err(failure("Capture interrupted before selection completed").into());
                }
            } else {
                self.collect_reader_capture(base, viewer, &cmd, options, filter).await?;
            }
            let capture = options.capture.take().ok_or_else(|| failure("Capture state unavailable"))?;
            self.finish_reader_capture(capture, options).await
        }
        .await;
        options.capture = None;
        self.session.search_pattern = search;
        if let Err(error) = result {
            log::error!("Reader capture failed: {error}");
            // Do not disclose host paths from I/O errors to the remote caller.
            self.println(
                TerminalTarget::Both,
                "Capture failed (selection, size limit, packet or transfer error); no incomplete packet will be sent.",
            )
            .await?;
            self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
        }
        Ok(())
    }

    pub(super) async fn collect_reader_capture(
        &mut self,
        base: &mut JamMessageBase,
        viewer: &MessageViewer,
        cmd: &ReadCommand,
        options: &mut ReaderOptions,
        inherited: Option<MessageFilter>,
    ) -> Res<()> {
        let number = self.session.current_conference_number;
        let area_number = self.session.current_message_area;
        let conf = self.session.current_conference.clone();
        let area = conf
            .areas
            .as_ref()
            .and_then(|areas| areas.get(area_number))
            .ok_or_else(|| failure("Capture area unavailable"))?;
        if !self.session.user_command_level.cmd_r.session_can_access(&self.session)
            || !conf.required_security.session_can_access(&self.session)
            || !area.req_level_to_list.session_can_access(&self.session)
        {
            return Err(failure("Capture access denied").into());
        }
        let read_all = self
            .get_board()
            .await
            .config
            .sysop_command_level
            .read_all_mail
            .session_can_access(&self.session);
        let filter = match inherited {
            Some(filter) => filter,
            None => self.reader_filter(cmd).await,
        };
        base.read_jhr_header()?;
        let pointer = base
            .find_last_read(
                JamMessageBase::crc(&BString::from(self.session.user_name.as_str())),
                self.session.cur_user_id as u32,
            )?
            .map_or(0, |last| last.last_read_msg);
        if cmd.since && pointer >= base.highest_message_number() {
            return Ok(());
        }
        let capture = options.capture.as_mut().ok_or_else(|| failure("Capture state unavailable"))?;
        for (range_index, range) in cmd.numbers.iter().enumerate() {
            let mut range = *range;
            if cmd.since && range_index == 0 {
                range.first = i64::from(pointer) + 1;
            }
            let (first, last) = self.clamp_range(range, base.lowest_message_number(), base.highest_message_number());
            if first == 0 {
                continue;
            }
            let mut current = first;
            loop {
                if capture.full(number) {
                    return Ok(());
                }
                if self.session.request_logoff || self.session.disp_options.abort_printout {
                    return Err(failure("Capture interrupted").into());
                }
                capture.scanned += 1;
                if capture.scanned > MAX_SCAN {
                    return Err(failure("Capture scan exceeds the safety limit").into());
                }
                // Check permission and declared size BEFORE allocating a body.
                // A read transaction gives header, permissions and text one generation.
                let candidate = base.read_transaction(|base| {
                    let header = match base.read_header(current) {
                        Ok(header) => header,
                        // JAM ranges may contain holes. No body or effects for holes.
                        Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted | jamjam::jam::JamError::MessageNumberOutOfRange(..))) => return Ok(None),
                        Err(error) => return Err(error),
                    };
                    if !exportable(&header, &self.session.user_name, &self.session.alias_name, read_all) {
                        return Ok(None);
                    }
                    if header.txt_len as usize > MAX_MESSAGE_BYTES {
                        return Err(failure("Capture message exceeds the safety size limit").into());
                    }
                    let bytes = base.read_message_text(&header)?;
                    let text = match std::str::from_utf8(&bytes) {
                        Ok(text) => text.to_string(),
                        Err(_) => bytes.iter().map(|byte| codepages::tables::CP437_TO_UNICODE[*byte as usize]).collect(),
                    };
                    Ok(Some((header, text)))
                })?;
                if let Some((header, text)) = candidate {
                    if filter.matches(&header, &text, pointer) {
                        let personal = header.to().is_some_and(|to| {
                            to.to_string().trim().eq_ignore_ascii_case(&self.session.user_name)
                                || (!self.session.alias_name.is_empty() && to.to_string().trim().eq_ignore_ascii_case(&self.session.alias_name))
                        });
                        let effect = ReadEffect {
                            path: base.path().to_path_buf(),
                            conference: number,
                            area: area_number,
                            header,
                        };
                        capture.append(
                            effect,
                            &text,
                            viewer,
                            options.header,
                            &conf.name,
                            if area.qwk_name.is_empty() { &area.name } else { &area.qwk_name },
                            personal,
                        )?;
                    }
                }
                if capture.scanned % 128 == 0 {
                    self.check_time_left().await;
                    tokio::task::yield_now().await;
                }
                match next_in_range(current, first, last) {
                    Some(next) => current = next,
                    None => break,
                }
            }
        }
        Ok(())
    }

    async fn finish_reader_capture(&mut self, capture: ReaderCapture, options: &ReaderOptions) -> Res<()> {
        if capture.limited {
            self.println(
                TerminalTarget::Both,
                "Capture stopped at the configured message limit; remaining messages were not exported.",
            )
            .await?;
        }
        if capture.effects.is_empty() {
            return self.display_text(IceText::CaptureFileIsEmpty, display_flags::NEWLINE).await;
        }
        if capture.attachments {
            self.println(TerminalTarget::Both, "Message bodies only: attachments are not included in reader captures.")
                .await?;
        }
        self.display_text(IceText::TotalMessagesInCapture, display_flags::NEWLINE).await?;
        self.println(TerminalTarget::Both, &capture.effects.len().to_string()).await?;
        if capture.ask {
            let answer = self
                .input_field(
                    IceText::DownloadTagged,
                    1,
                    "",
                    "",
                    Some(self.session.yes_char.to_string()),
                    display_flags::YESNO | display_flags::UPCASE | display_flags::NEWLINE | display_flags::FIELDLEN,
                )
                .await?;
            if answer.eq_ignore_ascii_case(&self.session.no_char.to_string()) {
                return Ok(());
            }
        }
        if self.session.request_logoff {
            return Ok(());
        }
        let packet = capture.packet()?;
        if !self.send_reader_capture(&packet.path).await? {
            self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
            return Ok(());
        }
        let update_pointer = options.update_pointers && self.get_board().await.config.message.update_last_read_pointer;
        self.display_text(IceText::TransferSuccessful, display_flags::NEWLINE).await?;
        let mut effects_complete = true;
        // Read effects are never applied to cancelled/failed/empty captures.
        for effect in &capture.effects {
            let committed: Res<_> = (|| {
                let mut base = JamMessageBase::open(&effect.path)?;
                commit_effect(
                    &mut base,
                    &effect.header,
                    &self.session.user_name,
                    &self.session.alias_name,
                    self.session.cur_user_id as u32,
                    options.update_status,
                    update_pointer,
                )
            })();
            let (unchanged, pointers) = match committed {
                Ok(result) => result,
                Err(error) => {
                    log::error!("Delivered capture: cannot commit message read effects: {error}");
                    self.println(TerminalTarget::Both, "Download completed, but some message read state could not be saved.")
                        .await?;
                    effects_complete = false;
                    break;
                }
            };
            if !unchanged {
                effects_complete = false;
                self.println(
                    TerminalTarget::Both,
                    "A captured message changed during transfer; its read state was not updated.",
                )
                .await?;
                continue;
            }
            if let Some(pointers) = pointers {
                if effect.conference == self.session.current_conference_number && effect.area == self.session.current_message_area {
                    (self.session.last_msg_read, self.session.highest_msg_read) = pointers;
                }
            }
        }
        if capture.bye && effects_complete {
            self.goodbye().await?;
        }
        Ok(())
    }

    /// Deliberately not download(): that method drains unrelated flagged files
    /// and returns Ok even for failed transfers, so it cannot commit read effects.
    async fn send_reader_capture(&mut self, path: &Path) -> Res<bool> {
        let Some(user) = self.session.current_user.as_ref() else {
            return Ok(false);
        };
        if !self.session.user_command_level.cmd_d.session_can_access(&self.session) {
            return Ok(false);
        }
        let history = TransferHistory {
            num_uploads: user.stats.num_uploads,
            num_downloads: user.stats.num_downloads,
            total_upld_bytes: user.stats.total_upld_bytes,
            total_dnld_bytes: user.stats.total_dnld_bytes,
        };
        let default = user.protocol.clone();
        let size = std::fs::metadata(path)?.len();
        let mut limits = self.session.transfer_limits.clone();
        limits.bytes_remaining = (self.session.bytes_remaining >= 0).then_some(self.session.bytes_remaining);
        if self.get_board().await.config.system_control.enforce_transfer_limits && !limits.check_file(&history, BatchSoFar::default(), size, false).is_allowed()
        {
            self.println(TerminalTarget::Both, "Capture download exceeds your transfer allowance.").await?;
            return Ok(false);
        }
        let seconds = limits::seconds_for_transfer(size, self.get_bps().max(0) as u32);
        if self.minutes_left().is_some_and(|minutes| seconds > (minutes - 1) * 60) {
            self.display_text(IceText::NoTimeForDownload, display_flags::NEWLINE).await?;
            return Ok(false);
        }
        if self.session.is_local {
            // Keep the same access/allowance gates and offer only the owned packet.
            // The bridge additionally requires the local console's picker capability.
            let started = std::time::Instant::now();
            let state = match self.local_download_files(&[path.to_path_buf()]).await {
                Ok(Some(state)) => state,
                Ok(None) => return Ok(false),
                Err(error) => {
                    log::error!("Local reader capture failed: {error}");
                    // finish_reader_capture displays TransferAborted without host paths.
                    return Ok(false);
                }
            };
            if self.session.request_logoff || !delivered(&state, path) {
                return Ok(false);
            }
            return self.record_reader_capture_download(path, size, &state, "Local", started).await;
        }
        let answer = self.ask_transfer_protocol(&default).await?;
        if answer.is_empty() || answer.eq_ignore_ascii_case("N") || self.session.request_logoff {
            return Ok(false);
        }
        let protocol = self
            .get_board()
            .await
            .protocols
            .iter()
            .find(|protocol| protocol.is_enabled && protocol.char_code.eq_ignore_ascii_case(&answer))
            .and_then(|protocol| create_protocol(&protocol.send_command));
        let Some(mut protocol) = protocol else {
            return Ok(false);
        };
        self.display_text(IceText::SendingFiles, display_flags::NEWLINE).await?;
        let deadline = Instant::now() + Duration::from_secs(300);
        let files = vec![path.to_path_buf()];
        let mut state = match timeout_at(deadline, protocol.initiate_send(&mut *self.connection, &files)).await {
            Ok(Ok(state)) => state,
            _ => {
                let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
                return Ok(false);
            }
        };
        let started = std::time::Instant::now();
        let mut failed = false;
        while !state.is_finished && !state.request_cancel && !self.session.request_logoff {
            self.check_time_left().await;
            if self.session.request_logoff {
                break;
            }
            if !matches!(
                timeout_at(deadline, protocol.update_transfer(&mut *self.connection, &mut state)).await,
                Ok(Ok(()))
            ) {
                failed = true;
                break;
            }
        }
        let success = !failed && !self.session.request_logoff && delivered(&state, path);
        if !success {
            let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
            return Ok(false);
        }
        self.record_reader_capture_download(path, size, &state, &answer, started).await
    }

    async fn record_reader_capture_download(
        &mut self,
        path: &Path,
        size: u64,
        state: &icy_net::protocol::TransferState,
        protocol: &str,
        started: std::time::Instant,
    ) -> Res<bool> {
        self.transfer_statistics.downloaded_bytes = size as usize;
        self.transfer_statistics.downloaded_files = 1;
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_downloads = user.stats.num_downloads.saturating_add(1);
            user.stats.today_num_downloads = user.stats.today_num_downloads.saturating_add(1);
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_add(size);
            user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_add(size as i64);
        }
        limits::adjust_bytes_remaining(&mut self.session.bytes_remaining, size as i64);
        let sent = vec![path.file_name().unwrap_or_default().to_string_lossy().to_string()];
        let cps = crate::icy_board::state::functions::transfer_cps(state.send_state.total_bytes_transfered, started);
        // Logging failures do not turn a confirmed delivery into a failed
        // transfer or suppress the pending message-pointer commit.
        let log_result = self.log_transfer(false, &sent, protocol, state.send_state.errors, cps).await;
        let statistics_result = {
            let mut board = self.get_board().await;
            board.statistics.add_download(state);
            board.save_statistics()
        };
        if let Err(error) = &log_result {
            log::error!("Delivered capture: transfer log failed: {error}");
        }
        if let Err(error) = &statistics_result {
            log::error!("Delivered capture: statistics save failed: {error}");
        }
        if log_result.is_err() || statistics_result.is_err() {
            self.println(TerminalTarget::Both, "Download completed, but transfer logging/statistics could not be saved.")
                .await?;
        }
        Ok(true)
    }
}

fn delivered(state: &icy_net::protocol::TransferState, path: &Path) -> bool {
    state.is_finished && !state.request_cancel && state.send_state.finished_files.len() == 1 && state.send_state.finished_files[0].1 == path
}

fn same_message(a: &JamMessageHeader, b: &JamMessageHeader) -> bool {
    !a.is_deleted()
        && a.message_number == b.message_number
        && a.msgid_crc == b.msgid_crc
        && a.date_written == b.date_written
        && a.offset == b.offset
        && a.txt_len == b.txt_len
        && a.password_crc == b.password_crc
        && a.attributes & !attributes::MSG_READ == b.attributes & !attributes::MSG_READ
        && a.sub_fields.len() == b.sub_fields.len()
        && a.sub_fields
            .iter()
            .zip(&b.sub_fields)
            .all(|(a, b)| a.field_type() == b.field_type() && a.content() == b.content())
}

/// JAM locks nest on the same handle. The snapshot test, receipt operation and
/// pointer advance therefore share one exclusive transaction, without a race
/// against a concurrent editor/packer between the test and the writes.
fn commit_effect(
    base: &mut JamMessageBase,
    snapshot: &JamMessageHeader,
    user: &str,
    alias: &str,
    user_id: u32,
    status: bool,
    pointer: bool,
) -> Res<(bool, Option<(u32, u32)>)> {
    Ok(base.transaction(|base| {
        let result: Res<_> = (|| {
            let fresh = match base.read_header(snapshot.message_number) {
                Ok(header) => header,
                Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted | jamjam::jam::JamError::MessageNumberOutOfRange(..))) => {
                    return Ok((false, None));
                }
                Err(error) => return Err(error.into()),
            };
            if !same_message(&fresh, snapshot) {
                return Ok((false, None));
            }
            if status {
                record_recipient_read(base, fresh.message_number, user, alias)?;
            }
            let pointers = if pointer {
                Some(advance_read_pointer(base, user, user_id, fresh.message_number)?)
            } else {
                None
            };
            Ok((true, pointers))
        })();
        result.map_err(|error| io::Error::other(error.to_string()).into())
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::icb_text::IcbTextFile;
    use jamjam::jam::{JamMessage, msg_header::MessageSubfield};
    use std::io::{Cursor, Read};

    fn control() -> ControlDat {
        ControlDat {
            bbs_name: "Board".into(),
            bbs_city_and_state: "City".into(),
            bbs_phone_number: "Phone".into(),
            bbs_sysop_name: "Sysop".into(),
            bbs_id: "../../never-a-path".into(),
            serial_number: 0,
            creation_time: "09/06/26,12:00".into(),
            qmail_user_name: "READER".into(),
            qmail_menu_name: BString::default(),
            zero_line: "0".into(),
            message_count: 0,
            conferences: Vec::new(),
            welcome_screen: BString::default(),
            news_screen: BString::default(),
            logoff_screen: BString::default(),
            extra_lines: Vec::new(),
        }
    }

    fn capture(qwk: bool, zip: bool) -> ReaderCapture {
        ReaderCapture::new(
            &ReadCommand {
                open_qwk: qwk,
                open_capture: !qwk,
                zip_cap: zip,
                ..Default::default()
            },
            control(),
            HashMap::from([((0, 0), 300)]),
            600,
            200,
        )
    }

    fn header(number: u32) -> JamMessageHeader {
        let message = JamMessage::default()
            .with_from("AUTHOR".into())
            .with_to("READER".into())
            .with_subject("Subject".into());
        let mut header = message.header().clone();
        header.message_number = number;
        header
    }

    fn append(capture: &mut ReaderCapture, header: JamMessageHeader, body: &str, personal: bool) -> Res<()> {
        let viewer = MessageViewer::load(&IcbTextFile::default())?;
        capture.append(
            ReadEffect {
                path: PathBuf::from("unused-test-base"),
                conference: 0,
                area: 0,
                header,
            },
            body,
            &viewer,
            HeaderLength::Long,
            "Main",
            "General",
            personal,
        )
    }

    #[test]
    fn zero_user_limit_means_system_limit_not_empty_capture() {
        assert_eq!(message_limit(600, 0), 600);
        assert_eq!(message_limit(600, 900), 600);
        assert_eq!(message_limit(600, 10), 10);
        assert_eq!(message_limit(0, 10), 0);
    }

    mod local_capture_tests {
        use super::*;
        use crate::icy_board::{
            IcyBoard,
            bbs::BBS,
            security_expr::SecurityExpression,
            state::local_transfer::{LocalFilePickerKind, LocalFilePickerRequest},
            user_base::User,
        };
        use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
        use std::sync::Arc;
        use tokio::sync::{Mutex, mpsc};

        async fn fixture(root: &Path) -> (IcyBoardState, ChannelConnection, mpsc::Receiver<LocalFilePickerRequest>) {
            let bbs = Arc::new(Mutex::new(BBS::new(1)));
            let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
            let nodes = bbs.lock().await.open_connections.clone();
            let (peer, connection) = ChannelConnection::create_pair();
            let mut board = IcyBoard::new();
            board.config.paths.statistics_file = root.join("statistics.toml");
            board.config.paths.transfer_log = root.join("transfers.log");
            board.config.switches.exclude_local_calls_stats = false;
            board.config.system_control.enforce_transfer_limits = false;
            board.config.message.update_last_read_pointer = true;
            let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
            state.session.current_user = Some(User {
                name: "READER".into(),
                // No usable protocol and no terminal input: local must bypass selection.
                protocol: "N".into(),
                ..Default::default()
            });
            state.session.user_name = "READER".into();
            state.session.cur_user_id = 1;
            state.session.cur_security = 10;
            state.session.user_command_level.cmd_d = SecurityExpression::from_req_security(0);
            state.session.is_local = true;
            state.session.is_sysop = false;
            state.session.page_len = 0;
            state.session.time_limit = 0;
            state.session.bytes_remaining = 1_000_000;
            let unrelated = root.join("unrelated.bin");
            std::fs::write(&unrelated, b"not offered").unwrap();
            state.session.flagged_files.push(unrelated);
            let (sender, receiver) = mpsc::channel(1);
            state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(sender);
            (state, peer, receiver)
        }

        async fn answer(picker: &mut mpsc::Receiver<LocalFilePickerRequest>, destination: Option<PathBuf>) {
            let request = picker.recv().await.unwrap();
            assert_eq!(request.kind, LocalFilePickerKind::DownloadDirectory);
            request.response.send(destination).unwrap();
        }

        #[tokio::test]
        async fn send_confirms_copy_or_cancel_and_cleans_owned_packet() {
            for outcome in ["success", "cancel", "collision", "error"] {
                let root = tempfile::tempdir().unwrap();
                let destination = tempfile::tempdir().unwrap();
                let (mut state, _peer, mut picker) = fixture(root.path()).await;
                let flags = state.session.flagged_files.clone();
                let mut capture = capture(false, false);
                append(&mut capture, header(1), "Local capture café\nSecond line", true).unwrap();
                let packet = capture.packet().unwrap();
                let source = packet.path.clone();
                let directory = source.parent().unwrap().to_path_buf();
                let expected = std::fs::read(&source).unwrap();
                let target = destination.path().join("messages.txt");
                if outcome == "collision" {
                    std::fs::write(&target, b"existing").unwrap();
                }
                let chosen = match outcome {
                    "cancel" => None,
                    "error" => Some(destination.path().join("missing-directory")),
                    _ => Some(destination.path().to_path_buf()),
                };
                let (result, ()) = timeout(Duration::from_secs(5), async {
                    tokio::join!(state.send_reader_capture(&source), answer(&mut picker, chosen))
                })
                .await
                .expect("local capture send stalled");
                let success = outcome == "success";
                assert_eq!(result.unwrap(), success, "{outcome}");
                assert_eq!(state.session.flagged_files, flags);
                assert_eq!(std::fs::read(&flags[0]).unwrap(), b"not offered");
                assert!(!destination.path().join("unrelated.bin").exists());
                assert_eq!((state.session.last_msg_read, state.session.highest_msg_read), (0, 0));
                let user = state.session.current_user.as_ref().unwrap();
                assert_eq!(user.stats.num_downloads, u64::from(success));
                assert_eq!(user.stats.total_dnld_bytes, if success { expected.len() as u64 } else { 0 });
                assert_eq!(state.session.bytes_remaining, 1_000_000 - if success { expected.len() as i64 } else { 0 });
                assert_eq!(root.path().join("transfers.log").exists(), success);
                if success {
                    assert_eq!(std::fs::read(&target).unwrap(), expected);
                    assert_eq!(std::fs::read_dir(destination.path()).unwrap().count(), 1);
                } else if outcome == "collision" {
                    assert_eq!(std::fs::read(&target).unwrap(), b"existing");
                    assert_eq!(std::fs::read_dir(destination.path()).unwrap().count(), 1);
                } else {
                    assert_eq!(std::fs::read_dir(destination.path()).unwrap().count(), 0);
                }
                assert_eq!(std::fs::read(&source).unwrap(), expected);
                drop(packet);
                assert!(!source.exists());
                assert!(!directory.exists());
                if success {
                    assert_eq!(std::fs::read(&target).unwrap(), expected);
                }
            }
        }

        #[tokio::test]
        async fn finish_commits_only_delivered_selected_mail_and_obeys_reader_options() {
            for (outcome, status, pointer) in [
                ("success", true, true),
                ("success", false, false),
                ("success", true, false),
                ("success", false, true),
                ("cancel", true, true),
                ("error", true, true),
                ("collision", true, true),
            ] {
                let root = tempfile::tempdir().unwrap();
                let destination = tempfile::tempdir().unwrap();
                let (mut state, mut peer, mut picker) = fixture(root.path()).await;
                let flags = state.session.flagged_files.clone();
                let mut base = JamMessageBase::create(root.path().join("mail")).unwrap();
                for body in ["selected café", "unrelated message"] {
                    base.write_message(
                        &JamMessage::default()
                            .with_from("AUTHOR".into())
                            .with_to("READER".into())
                            .with_attributes(attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ)
                            .with_text(body.into()),
                    )
                    .unwrap();
                }
                base.write_jhr_header().unwrap();
                let mut capture = capture(false, false);
                append(&mut capture, base.read_header(1).unwrap(), "selected café", true).unwrap();
                capture.effects[0].path = base.path().to_path_buf();
                let expected = capture.data.clone();
                let options = ReaderOptions {
                    update_status: status,
                    update_pointers: pointer,
                    ..Default::default()
                };
                if outcome == "collision" {
                    std::fs::write(destination.path().join("messages.txt"), b"existing").unwrap();
                }
                let chosen = match outcome {
                    "cancel" => None,
                    "error" => Some(destination.path().join("missing-directory")),
                    _ => Some(destination.path().to_path_buf()),
                };
                let (result, ()) = timeout(Duration::from_secs(5), async {
                    tokio::join!(state.finish_reader_capture(capture, &options), answer(&mut picker, chosen))
                })
                .await
                .expect("local capture finish stalled");
                result.unwrap();
                let success = outcome == "success";
                let mut base = JamMessageBase::open(base.path()).unwrap();
                let selected = base.read_header(1).unwrap();
                assert_eq!(selected.is_read(), success && status);
                assert_eq!(selected.is_receipt_req(), !(success && status));
                assert_eq!(base.highest_message_number(), if success && status { 3 } else { 2 });
                assert!(!base.read_header(2).unwrap().is_read());
                assert!(base.read_header(2).unwrap().is_receipt_req());
                let last = base.find_last_read(JamMessageBase::crc(&"READER".into()), 1).unwrap();
                assert_eq!(
                    last.map(|last| (last.last_read_msg, last.high_read_msg)),
                    (success && pointer).then_some((1, 1))
                );
                assert_eq!(
                    (state.session.last_msg_read, state.session.highest_msg_read),
                    if success && pointer { (1, 1) } else { (0, 0) }
                );
                assert_eq!(state.session.flagged_files, flags);
                assert!(!state.session.request_logoff);
                if success {
                    assert_eq!(std::fs::read(destination.path().join("messages.txt")).unwrap(), expected);
                }
                let mut output = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let count = peer.try_read(&mut buffer).await.unwrap();
                    if count == 0 {
                        break;
                    }
                    output.extend_from_slice(&buffer[..count]);
                }
                let output = String::from_utf8_lossy(&output).to_lowercase();
                assert_eq!(output.contains("transfer aborted"), !success, "{output}");
            }
        }

        #[tokio::test]
        async fn local_copy_keeps_login_access_allowance_and_capability_gates() {
            for gate in ["login", "access", "allowance", "capability"] {
                let root = tempfile::tempdir().unwrap();
                let (mut state, _peer, mut picker) = fixture(root.path()).await;
                match gate {
                    "login" => state.session.current_user = None,
                    "access" => state.session.user_command_level.cmd_d = SecurityExpression::from_req_security(20),
                    "allowance" => {
                        state.get_board().await.config.system_control.enforce_transfer_limits = true;
                        state.session.bytes_remaining = 0;
                    }
                    _ => state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = None,
                }
                let mut capture = capture(false, false);
                append(&mut capture, header(1), "not offered", false).unwrap();
                let packet = capture.packet().unwrap();
                assert!(
                    !timeout(Duration::from_secs(5), state.send_reader_capture(&packet.path))
                        .await
                        .expect("denied local capture stalled")
                        .unwrap(),
                    "{gate}"
                );
                assert!(picker.try_recv().is_err(), "{gate}");
                assert_eq!(state.session.flagged_files, vec![root.path().join("unrelated.bin")]);
                assert!(!root.path().join("transfers.log").exists());
            }
        }
    }

    #[tokio::test]
    async fn native_zmodem_capture_delivers_exact_packet_and_cleans_cancelled_transfer() {
        use crate::icy_board::{
            IcyBoard,
            bbs::BBS,
            state::{KeyChar, KeySource},
            user_base::User,
            xfer_protocols::SupportedProtocols,
        };
        use icy_net::{
            ConnectionType,
            channel::ChannelConnection,
            protocol::{Protocol, Zmodem},
        };
        use std::sync::Arc;
        use tempfile::TempPath;
        use tokio::sync::Mutex;

        for cancel in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let bbs = Arc::new(Mutex::new(BBS::new(1)));
            let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
            let nodes = bbs.lock().await.open_connections.clone();
            let (mut peer, connection) = ChannelConnection::create_pair();
            let mut board = IcyBoard::new();
            // Bare boards have no configured protocols, including Zmodem.
            board.protocols = SupportedProtocols::generate_pcboard_defaults();
            board.config.paths.statistics_file = root.path().join("statistics.toml");
            board.config.paths.transfer_log = root.path().join("transfers.log");
            board.config.system_control.enforce_transfer_limits = false;
            let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
            state.session.current_user = Some(User {
                name: "READER".into(),
                protocol: "Z".into(),
                ..Default::default()
            });
            state.session.user_name = "READER".into();
            state.session.cur_security = 255;
            state.session.page_len = 0;
            state.session.bytes_remaining = 1_000_000;
            state.char_buffer.extend("Z\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));

            let mut capture = capture(false, false);
            append(&mut capture, header(1), "Native capture café\nSecond line", false).unwrap();
            let packet = capture.packet().unwrap();
            let path = packet.path.clone();
            let directory = path.parent().unwrap().to_path_buf();
            let expected = std::fs::read(&path).unwrap();
            assert_eq!(expected, capture.data);
            let original_downloads = state.session.current_user.as_ref().unwrap().stats.num_downloads;
            let original_bytes = state.session.current_user.as_ref().unwrap().stats.total_dnld_bytes;

            let (result, received) = timeout(Duration::from_secs(10), async {
                tokio::join!(state.send_reader_capture(&path), async {
                    let mut protocol = Zmodem::new(1024);
                    let mut received = protocol.initiate_recv(&mut peer).await.unwrap();
                    let mut files = Vec::new();
                    while !received.is_finished {
                        protocol.update_transfer(&mut peer, &mut received).await.unwrap();
                        // Adopt completed receive files before any assertion or
                        // await, so even a failed test does not leak them.
                        for (name, path) in received.recieve_state.finished_files.drain(..) {
                            files.push((name, TempPath::try_from_path(path).unwrap()));
                        }
                        if cancel && received.recieve_state.file_size > 0 {
                            // The real ZFILE was accepted and its receive temp
                            // opened. Cancel before any data/EOF is accepted.
                            assert_eq!(received.recieve_state.file_name, "messages.txt");
                            assert!(files.is_empty());
                            protocol.cancel_transfer(&mut peer).await.unwrap();
                            return files;
                        }
                        tokio::task::yield_now().await;
                    }
                    files
                })
            })
            .await
            .expect("native capture download stalled");
            assert_eq!(result.unwrap(), !cancel);
            let user = state.session.current_user.as_ref().unwrap();
            if cancel {
                assert!(received.is_empty());
                assert_eq!(user.stats.num_downloads, original_downloads);
                assert_eq!(user.stats.total_dnld_bytes, original_bytes);
                assert_eq!(state.session.bytes_remaining, 1_000_000);
                assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
            } else {
                assert_eq!(received.len(), 1);
                assert_eq!(received[0].0, "messages.txt");
                assert_ne!(received[0].1.to_path_buf(), path);
                assert_eq!(std::fs::read(&received[0].1).unwrap(), expected);
                assert_eq!(user.stats.num_downloads, original_downloads + 1);
                assert_eq!(user.stats.total_dnld_bytes, original_bytes + expected.len() as u64);
                assert_eq!(state.transfer_statistics.downloaded_files, 1);
                assert_eq!(state.transfer_statistics.downloaded_bytes, expected.len());
                assert_eq!(state.session.bytes_remaining, 1_000_000 - expected.len() as i64);
            }
            let received_paths: Vec<_> = received.iter().map(|(_, path)| path.to_path_buf()).collect();
            drop(received);
            assert!(received_paths.iter().all(|path| !path.exists()));
            drop(packet);
            assert!(!path.exists());
            assert!(!directory.exists());
        }
    }

    #[test]
    fn capture_flags_are_supported_or_explicitly_rejected() {
        use super::super::read_command::{ParseContext, ReadLoop, parse};
        for (line, accepted) in [
            ("1 C", true),
            ("1 D", true),
            ("1 Z", true),
            ("1 QWK", true),
            ("1 QWK BYE", true),
            ("1 QWK NET", false),
            ("NET", false),
            ("BYE", false),
            ("1 Q C", false),
        ] {
            let tokens = line.split_whitespace().map(str::to_string).collect::<Vec<_>>();
            let cmd = parse(
                &tokens,
                ReadLoop::Outside,
                &ParseContext {
                    qwk_support: true,
                    may_quick_scan: true,
                    ..Default::default()
                },
            );
            assert_eq!(unsupported(&cmd).is_none(), accepted, "{line}");
        }
    }

    #[test]
    fn independent_security_gate_blocks_other_users_private_and_group_password_mail() {
        let mut private = header(1);
        private.attributes |= attributes::MSG_PRIVATE;
        assert!(!exportable(&private, "STRANGER", "", false));
        assert!(exportable(&private, "reader", "", false));
        assert!(exportable(&private, "AUTHOR", "", false));
        assert!(exportable(&private, "STRANGER", "reader", false));
        assert!(exportable(&private, "SYSOP", "", true));
        private.attributes |= attributes::MSG_DELETED;
        assert!(!exportable(&private, "SYSOP", "", true));
        let group = JamMessage::default()
            .with_from("AUTHOR".into())
            .with_to("ALL".into())
            .with_password(&"SECRET".into())
            .with_sub_field(MessageSubfield::new(SubfieldType::FTSKludge, "ICYBOARD-SECURITY: G".into()));
        assert!(!exportable(group.header(), "READER", "", false));
        assert!(exportable(group.header(), "SYSOP", "", true));
    }

    #[test]
    fn qwk_packet_has_exact_selected_order_valid_indexes_and_private_status() {
        let mut capture = capture(true, true);
        let mut private = header(9);
        private.attributes |= attributes::MSG_PRIVATE;
        append(&mut capture, private, "Body nine\r\nSecond line", true).unwrap();
        append(&mut capture, header(3), "Body three", false).unwrap();
        // Overlapping ranges must not create duplicate messages or receipts.
        append(&mut capture, header(9), "duplicate", true).unwrap();
        let packet = capture.packet().unwrap();
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&packet.path).unwrap()).unwrap();
        assert_eq!(
            archive.file_names().collect::<Vec<_>>(),
            ["CONTROL.DAT", "MESSAGES.DAT", "300.NDX", "PERSONAL.NDX"]
        );
        let mut control = Vec::new();
        archive.by_name("CONTROL.DAT").unwrap().read_to_end(&mut control).unwrap();
        let control = ControlDat::read(&control).unwrap();
        assert_eq!(control.message_count, 2);
        assert_eq!(control.conferences.len(), 1);
        assert_eq!(control.conferences[0].number, 300);
        let mut data = Vec::new();
        archive.by_name("MESSAGES.DAT").unwrap().read_to_end(&mut data).unwrap();
        assert_eq!(data.len() % 128, 0);
        let mut reader = Cursor::new(&data[128..]);
        let first = QwkMessage::read(&mut reader, true).unwrap();
        assert_eq!(first.msg_number, 9);
        assert_eq!(first.status, b'*');
        assert_eq!(first.conference_number, 300);
        assert!(first.password.is_empty());
        assert!(first.text.to_string().contains("Body nine"));
        let next_block = (reader.position() as usize + 128) / 128 + 1;
        assert_eq!(QwkMessage::read(&mut reader, true).unwrap().msg_number, 3);
        let mut index = Vec::new();
        archive.by_name("300.NDX").unwrap().read_to_end(&mut index).unwrap();
        assert_eq!(&index[..4], BasicReal::from(2i32).bytes());
        assert_eq!(&index[5..9], BasicReal::from(next_block as i32).bytes());
        assert_eq!(index[4], 255);
        let mut personal = Vec::new();
        archive.by_name("PERSONAL.NDX").unwrap().read_to_end(&mut personal).unwrap();
        assert_eq!(personal, index[..5]);
    }

    #[test]
    fn text_and_zip_use_only_owned_paths_and_cleanup_on_drop() {
        for zipped in [false, true] {
            let mut capture = capture(false, zipped);
            append(&mut capture, header(1), "Text body\nLast line", false).unwrap();
            let packet = capture.packet().unwrap();
            let path = packet.path.clone();
            let directory = path.parent().unwrap().to_path_buf();
            assert_eq!(path.file_name().unwrap(), if zipped { "messages.zip" } else { "messages.txt" });
            let data = if zipped {
                let mut archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
                assert_eq!(archive.len(), 1);
                let mut data = Vec::new();
                archive.by_name("MESSAGES.TXT").unwrap().read_to_end(&mut data).unwrap();
                data
            } else {
                std::fs::read(&path).unwrap()
            };
            assert!(data.windows(b"Text body\r\nLast line".len()).any(|part| part == b"Text body\r\nLast line"));
            assert!(data.ends_with(b"\r\n<<<>>>\r\n"));
            drop(packet);
            assert!(!path.exists());
            assert!(!directory.exists());
        }
    }

    #[tokio::test]
    async fn cancellation_drops_the_scoped_packet() {
        let mut capture = capture(false, false);
        append(&mut capture, header(1), "private body", false).unwrap();
        let packet = capture.packet().unwrap();
        let path = packet.path.clone();
        let (ready, wait) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _packet = packet;
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        wait.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!path.exists());
    }

    #[test]
    fn oversized_message_or_capture_fails_without_appending() {
        let mut capture = capture(true, true);
        let original = capture.data.clone();
        assert!(append(&mut capture, header(1), &"x".repeat(MAX_MESSAGE_BYTES + 1), false).is_err());
        assert_eq!(capture.data, original);
        assert!(capture.effects.is_empty());
        capture.memory = MAX_BYTES;
        assert!(append(&mut capture, header(1), "small", false).is_err());
        assert_eq!(capture.data, original);
        assert!(capture.effects.is_empty());
    }

    #[test]
    fn count_limit_is_explicit_and_does_not_add_later_effects() {
        let mut capture = capture(true, true);
        capture.max_per_conf = 1;
        append(&mut capture, header(1), "one", false).unwrap();
        append(&mut capture, header(2), "two", false).unwrap();
        assert!(capture.limited);
        assert_eq!(capture.effects.len(), 1);
        assert_eq!(capture.control.message_count, 1);
    }

    #[test]
    fn only_confirmed_exact_file_delivery_counts_as_success() {
        let path = Path::new("packet.qwk");
        let mut state = icy_net::protocol::TransferState::new("test".into());
        state.is_finished = true;
        assert!(!delivered(&state, path));
        state.send_state.finished_files.push(("other.qwk".into(), "other.qwk".into()));
        assert!(!delivered(&state, path));
        state.send_state.finished_files[0].1 = path.to_path_buf();
        assert!(delivered(&state, path));
        state.request_cancel = true;
        assert!(!delivered(&state, path));
    }

    #[test]
    fn capture_encoding_is_cp437_and_cannot_inject_control_records() {
        assert_eq!(field("a\r\nb\x1bc"), BString::from("abc"));
        assert_eq!(body_bytes("café\r\n\x1b[31m"), BString::from(b"caf\x82\n[31m".as_slice()));
    }

    #[test]
    fn read_effects_obey_o_and_status_options_and_reject_replaced_messages() {
        let directory = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(directory.path().join("mail")).unwrap();
        base.write_message(
            &JamMessage::default()
                .with_from("AUTHOR".into())
                .with_to("READER".into())
                .with_text("body".into()),
        )
        .unwrap();
        let snapshot = base.read_header(1).unwrap();
        assert_eq!(commit_effect(&mut base, &snapshot, "READER", "", 1, false, false).unwrap(), (true, None));
        assert!(!base.read_header(1).unwrap().is_read());
        assert!(base.find_last_read(JamMessageBase::crc(&"READER".into()), 1).unwrap().is_none());
        let mut stale = snapshot.clone();
        stale.msgid_crc ^= 1;
        assert_eq!(commit_effect(&mut base, &stale, "READER", "", 1, true, true).unwrap(), (false, None));
        assert!(!base.read_header(1).unwrap().is_read());
        assert_eq!(commit_effect(&mut base, &snapshot, "READER", "", 1, true, false).unwrap(), (true, None));
        assert!(base.read_header(1).unwrap().is_read());
        let snapshot = base.read_header(1).unwrap();
        assert_eq!(commit_effect(&mut base, &snapshot, "READER", "", 1, false, true).unwrap(), (true, Some((1, 1))));
    }
}
