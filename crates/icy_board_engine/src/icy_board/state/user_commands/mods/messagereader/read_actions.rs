//! The commands that act on the message in front of the reader.

use bstr::BString;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use jamjam::jam::msg_header::{JamMessageHeader, MessageSubfield, SubfieldType};
use jamjam::jam::{JamMessage, JamMessageBase, attributes, raw};

use crate::Res;
use crate::icy_board::icb_text::IceText;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::{MASK_ASCII, MASK_NUM, display_flags};
use crate::icy_board::state::user_commands::mods::editor::EditResult;
use crate::icy_board::state::user_commands::pcb::select_conferences::SelectMode;
use crate::icy_board::user_base::ConferenceFlags;
use crate::vm::TerminalTarget;

use super::message_security::{may_read_header, set_security_kind, requires_read_password};
use super::read_command::{MsgFunc, ReadCommand};

#[cfg(test)]
#[path = "read_action_tests.rs"]
mod read_action_tests;

/// Storage positions and numeric thread links belong to a base, not a message.
/// All other fields (including passwords, dates, unknown subfields and network
/// reply IDs) survive a copy unchanged.
fn transfer_draft(message: JamMessage, same_base: bool) -> JamMessage {
    let mut header = message.header().clone();
    header.message_number = 0;
    header.offset = 0;
    header.txt_len = 0;
    header.reply_first = 0;
    header.reply_next = 0;
    if !same_base {
        header.reply_to = 0;
    }
    JamMessage::from_stored(header, message.text().clone())
}

#[derive(Debug)]
enum TransferFailure {
    Destination(Box<dyn std::error::Error + Send + Sync>),
    Source(Box<dyn std::error::Error + Send + Sync>),
}

/// The destination may fail to open, write, or report completion. None of those
/// failures authorize removal of the source. A failed delete leaves two copies;
/// never attempt to undo a successful destination write by deleting that copy.
async fn finish_transfer(
    destination: impl Future<Output = Res<()>>,
    destination_path: &Path,
    moving: bool,
    delete_source: impl FnOnce() -> Res<()>,
) -> Result<(), TransferFailure> {
    destination.await.map_err(TransferFailure::Destination)?;
    if moving {
        sync_destination(destination_path).map_err(TransferFailure::Destination)?;
        delete_source().map_err(TransferFailure::Source)?;
    }
    Ok(())
}

/// JAM's ordinary append is not durable, and sync() tolerates missing files.
/// Require all three message files, locking out pack until their fsync finishes.
fn sync_destination(path: &Path) -> Res<()> {
    let mut base = JamMessageBase::open(path)?;
    base.read_transaction(|base| {
        for extension in ["jhr", "jdt", "jdx"] {
            std::fs::OpenOptions::new().write(true).open(base.path().with_extension(extension))?.sync_all()?;
        }
        std::fs::File::open(path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new(".")))?.sync_all()?;
        Ok(())
    }).map_err(Into::into)
}

fn compare_header(original: &JamMessageHeader, current: &JamMessageHeader) -> jamjam::Result<()> {
    let mut expected = Vec::new();
    let mut actual = Vec::new();
    original.write(&mut expected)?;
    current.write(&mut actual)?;
    if actual != expected {
        return Err(std::io::Error::other("Message changed during reader action").into());
    }
    Ok(())
}

/// Read positions are meaningful only within one locked generation. After a
/// password prompt, reject even a security-only change before reading the body.
fn action_snapshot(base: &mut JamMessageBase, number: u32, authorized: Option<&JamMessageHeader>) -> jamjam::Result<JamMessage> {
    base.read_transaction(|base| {
        let header = base.read_header(number)?;
        if let Some(authorized) = authorized { compare_header(authorized, &header)?; }
        let body = base.read_message_text(&header)?;
        Ok(JamMessage::from_stored(header, body))
    })
}

fn compare_message(base: &JamMessageBase, number: u32, original: &JamMessage) -> jamjam::Result<()> {
    let current = base.read_header(number)?;
    compare_header(original.header(), &current)?;
    if base.read_message_text(&current)? != *original.text() {
        return Err(std::io::Error::other("Message body changed during reader action").into());
    }
    Ok(())
}

fn delete_unchanged_message(base: &mut JamMessageBase, number: u32, original: &JamMessage) -> Res<()> {
    base.transaction(|base| {
        compare_message(base, number, original)?;
        base.delete_message(number)
    }).map_err(Into::into)
}

fn replace_header(base: &mut JamMessageBase, number: u32, original: &JamMessageHeader, draft: &JamMessageHeader) -> Res<()> {
    base.transaction(|base| {
        compare_header(original, &base.read_header(number)?)?;
        raw::update_header(base, number, draft)
    }).map_err(Into::into)
}

fn clear_password(header: &mut JamMessageHeader) {
    header.password_crc = JamMessageBase::crc(&BString::from(""));
    set_security_kind(header, false);
}

fn same_message_base(source: &Path, target: &Path) -> bool {
    source == target || match (source.with_extension("jhr").canonicalize(), target.with_extension("jhr").canonicalize()) {
        (Ok(source), Ok(target)) => source == target,
        _ => false,
    }
}

/// Replace the body without creating a second indexed message or changing its
/// number. Hold JAM's exclusive transaction across text append and raw header
/// replacement; a failed replacement leaves the original index/body intact.
fn replace_message(base: &mut JamMessageBase, number: u32, original: &JamMessage, draft: &JamMessage) -> Res<()> {
    base.transaction(|base| {
        compare_message(base, number, original)?;
        let mut text_file = std::fs::OpenOptions::new().append(true).open(base.path().with_extension("jdt"))?;
        let old_len = text_file.metadata()?.len();
        let end = old_len.checked_add(draft.text().len() as u64)
            .filter(|end| *end <= u32::MAX as u64)
            .ok_or_else(|| std::io::Error::other("JAM text file is full"))?;
        let mut header = draft.header().clone();
        header.offset = old_len as u32;
        header.txt_len = (end - old_len) as u32;
        header.message_number = number;
        let result = (|| {
            text_file.write_all(draft.text())?;
            text_file.sync_data()?;
            raw::update_header(base, number, &header)
        })();
        if result.is_err() {
            if let Err(error) = text_file.set_len(old_len) {
                log::error!("Could not reclaim failed EDIT text append: {error}");
            }
        }
        result
    }).map_err(Into::into)
}

/// JAM EnclFwAlias allows a NUL-separated display alias, but neither name is a
/// filesystem path or wildcard. Only regular files inside the configured root
/// may be offered; in particular a symlink must not escape the attachment area.
fn attachment_path(root: &Path, field: &MessageSubfield) -> Res<(PathBuf, String)> {
    let value = std::str::from_utf8(field.content())?;
    let mut parts = value.split('\0');
    let stored = parts.next().unwrap_or_default();
    let display = parts.next().unwrap_or(stored);
    let safe = |name: &str| !name.is_empty() && name != "." && name != ".."
        && !name.chars().any(|ch| ch.is_control() || "/\\:*?[]".contains(ch));
    if parts.next().is_some() || !safe(stored) || !safe(display) || root.as_os_str().is_empty() {
        return Err(std::io::Error::other("Invalid attachment filename").into());
    }
    let root = root.canonicalize()?;
    let path = root.join(stored);
    if !std::fs::symlink_metadata(&path)?.file_type().is_file() || path.canonicalize()?.parent() != Some(root.as_path()) {
        return Err(std::io::Error::other("Attachment is outside its conference").into());
    }
    Ok((path, display.to_string()))
}

/// Own only newly published copies, never pre-existing identical enclosures.
/// Commit at JAM append, before any statistics or terminal await can fail.
#[derive(Default)]
struct ActionAttachments {
    files: Vec<PathBuf>,
}

impl ActionAttachments {
    fn commit(&mut self) { self.files.clear(); }

    fn copy(&mut self, source: &Path, target: &Path, field: &MessageSubfield) -> Res<()> {
        let (path, _) = attachment_path(source, field)?;
        if source == target { return Ok(()); }
        let destination = target.join(path.file_name().ok_or_else(|| std::io::Error::other("Invalid attachment"))?);
        let mut staged = tempfile::NamedTempFile::new_in(target)?;
        std::io::copy(&mut std::fs::File::open(&path)?, &mut staged)?;
        staged.as_file().sync_all()?;
        match staged.persist_noclobber(&destination) {
            Ok(_) => {
                self.files.push(destination);
                std::fs::File::open(target)?.sync_all()?;
            }
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let (existing, _) = attachment_path(target, field)?;
                if !identical_files(error.file.path(), &existing)? {
                    return Err(std::io::Error::other("Destination attachment already exists with different contents").into());
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}

impl Drop for ActionAttachments {
    fn drop(&mut self) {
        for path in &self.files {
            if let Err(error) = std::fs::remove_file(path) {
                log::error!("Could not roll back copied attachment {}: {error}", path.display());
            }
        }
    }
}

fn identical_files(left: &Path, right: &Path) -> Res<bool> {
    use std::io::Read;
    let mut left = std::fs::File::open(left)?;
    let mut right = std::fs::File::open(right)?;
    let mut remaining = left.metadata()?.len();
    if remaining != right.metadata()?.len() { return Ok(false); }
    let mut a = [0; 8192];
    let mut b = [0; 8192];
    while remaining > 0 {
        let count = remaining.min(a.len() as u64) as usize;
        left.read_exact(&mut a[..count])?;
        right.read_exact(&mut b[..count])?;
        if a[..count] != b[..count] { return Ok(false); }
        remaining -= count as u64;
    }
    Ok(true)
}

fn after_existing_edit(result: EditResult) -> AfterAction {
    // PCBoard's SK is save-only for EDIT, not save-and-kill the existing mail.
    if result == EditResult::SendNext { AfterAction::Next } else { AfterAction::Redisplay }
}

/// Swaps one variable length header field for a new value.
fn replace_sub_field(header: &mut JamMessageHeader, field: SubfieldType, value: &str) {
    header.sub_fields.retain(|sub_field| sub_field.field_type() != field);
    header.sub_fields.push(MessageSubfield::new(field, BString::from(value)));
}

/// What the read loop should do once the command has run.
pub(super) enum AfterAction {
    /// Command not handled here.
    NotHandled,
    /// Ask for the next command without re-showing the message.
    Prompt,
    /// Show the message again (`PCBoard`'s REREAD).
    Redisplay,
    /// Move on to the next message (`PCBoard`'s READNEXT).
    Next,
    /// Leave the read loop (`PCBoard`'s QUITREAD/QUITLOOP/SKIPNEXT).
    Quit,
}

impl IcyBoardState {
    /// Actions may also be invoked at the outer prompt; never assume the body
    /// has already passed the reader's private/group-password checks.
    async fn read_action_message(&mut self, base: &mut JamMessageBase, number: u32) -> Res<Option<JamMessage>> {
        let original = match action_snapshot(base, number, None) {
            Ok(message) => message,
            Err(_) => {
                self.display_text(IceText::NoSuchMessageNumber, display_flags::NEWLINE).await?;
                return Ok(None);
            }
        };
        let header = original.header();
        let read_all = self.get_board().await.config.sysop_command_level.read_all_mail.session_can_access(&self.session);
        if header.is_deleted() || header.attributes & attributes::MSG_NODISP != 0
            || !may_read_header(header, &self.session.user_name, &self.session.alias_name, read_all) {
            self.display_text(IceText::NoSuchMessageNumber, display_flags::NEWLINE).await?;
            return Ok(None);
        }
        if requires_read_password(header, read_all)
            && !self.check_password(IceText::PasswordToReadMessage, 0, |password| header.is_password_valid(password)).await? {
            return Ok(None);
        }
        if self.session.request_logoff { return Ok(None); }
        // No JAM lock survives a terminal/board await. Recheck what was
        // authorized, then fetch the current header/body under one shared lock.
        match action_snapshot(base, number, Some(header)) {
            Ok(message) if message.text() == original.text() => Ok(Some(message)),
            _ => {
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                Ok(None)
            }
        }
    }

    async fn read_action_header(&mut self, base: &mut JamMessageBase, number: u32) -> Res<Option<JamMessageHeader>> {
        Ok(self.read_action_message(base, number).await?.map(|message| message.header().clone()))
    }

    fn action_conference_access(&self, number: u16, conference: &crate::icy_board::conferences::Conference) -> bool {
        let registered = self.session.current_user.as_ref()
            .and_then(|user| user.conference_flags.get(&(number as usize)))
            .is_some_and(|flags| flags.contains(ConferenceFlags::Registered));
        self.subscription_can_access_conference(number) && !self.is_lockedout(number)
            && conference.required_security.session_can_access(&self.session)
            && (number == self.session.current_conference_number || (
                self.session.user_command_level.cmd_j.session_can_access(&self.session)
                && (self.session.is_sysop || conference.is_public || registered)
                && (conference.password.is_empty() || self.session.joined_conferences.contains(&number))))
    }

    /// Bounds and authorization are checked before send_message's indexing, in
    /// the source session context (never grant the destination's security bonus).
    async fn read_action_target(&mut self, conference: u16, area: usize) -> Res<Option<PathBuf>> {
        let target = self.get_board().await.conferences.get(conference as usize).cloned();
        let Some(target) = target else { return Ok(None) };
        let Some(message_area) = target.areas.as_ref().and_then(|areas| areas.get(area)) else { return Ok(None) };
        if area > i32::MAX as usize || !self.action_conference_access(conference, &target)
            || !target.sec_write_message.session_can_access(&self.session)
            || !message_area.req_level_to_list.session_can_access(&self.session)
            || !message_area.req_level_to_enter.session_can_access(&self.session) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
            return Ok(None);
        }
        if target.is_read_only || message_area.is_read_only {
            self.display_text(IceText::ConferenceIsReadOnly, display_flags::NEWLINE).await?;
            return Ok(None);
        }
        if message_area.path.as_os_str().is_empty() { return Ok(None); }
        Ok(Some(self.resolve_path(&message_area.path)))
    }

    fn after_message_save(result: EditResult) -> AfterAction {
        match result {
            // Reply SK already deleted its authorized, unchanged source. A
            // second lookup can fail with MessageDeleted or hit a packed slot.
            EditResult::SendNext | EditResult::SendKill => AfterAction::Next,
            _ => AfterAction::Redisplay,
        }
    }

    async fn edit_read_message(&mut self, base: &mut JamMessageBase, number: u32) -> Res<AfterAction> {
        let sec = self.session.user_command_level.edit_own_messages.clone();
        if !self.check_sec("EDIT", &sec).await? { return Ok(AfterAction::Redisplay); }
        let Some(original) = self.read_action_message(base, number).await? else { return Ok(AfterAction::Redisplay) };
        let header = original.header();
        let own = header.from().is_some_and(|from| from.to_string().eq_ignore_ascii_case(&self.session.user_name)
            || (!self.session.alias_name.is_empty() && from.to_string().eq_ignore_ascii_case(&self.session.alias_name)));
        if !own && !self.get_board().await.config.sysop_command_level.edit_any_message.session_can_access(&self.session) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
            return Ok(AfterAction::Redisplay);
        }
        let mut draft = JamMessage::from_stored(header.clone(), original.text().clone());
        let mut attachments = self.message_attachment_cleanup(&original);
        let quote: Vec<String> = original.text().to_string().lines().map(str::to_string).collect();
        let result = loop {
            let result = self.edit_message_context(&mut draft, quote.clone()).await;
            attachments.track(&draft)?;
            let result = result?;
            if result == EditResult::Abort || self.session.request_logoff { return Ok(AfterAction::Redisplay); }
            if result != EditResult::AttachFile { break result; }
            let attached = self.attach_message_file(&mut draft).await;
            attachments.track(&draft)?;
            let attached = attached?;
            if self.session.request_logoff { return Ok(AfterAction::Redisplay); }
            if attached { break EditResult::SendMessage; }
            // Failed/cancelled attachment returns to the SAME draft, not storage.
        };
        if let Err(error) = replace_message(base, number, &original, &draft) {
            log::error!("Could not replace message {number}; original retained: {error}");
            self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
            return Ok(AfterAction::Redisplay);
        }
        attachments.commit();
        // SC edits this message only; EDIT must not create carbon-copy mail.
        Ok(after_existing_edit(result))
    }

    /// PCBoard FORWARD asks for a recipient, not a new body. The author, date,
    /// body, security and extension metadata survive; provenance is recorded.
    async fn forward_read_message(&mut self, cmd: &ReadCommand, base: &mut JamMessageBase, number: u32) -> Res<AfterAction> {
        let sec = self.session.user_command_level.cmd_e.clone();
        if !self.check_sec("FORWARD", &sec).await? { return Ok(AfterAction::Redisplay); }
        let source_conf = self.session.current_conference_number;
        if self.read_action_target(source_conf, self.session.current_message_area).await?.is_none() { return Ok(AfterAction::Redisplay); }
        let Some(original) = self.read_action_message(base, number).await? else { return Ok(AfterAction::Redisplay) };
        let header = original.header();
        let personal = header.to().is_some_and(|to| to.to_string().eq_ignore_ascii_case(&self.session.user_name)
            || (!self.session.alias_name.is_empty() && to.to_string().eq_ignore_ascii_case(&self.session.alias_name)));
        let move_sec = self.get_board().await.config.sysop_command_level.copy_move_messages.clone();
        let may_move = move_sec.session_can_access(&self.session);
        if !personal && !self.check_sec("FORWARD", &move_sec).await? { return Ok(AfterAction::Redisplay); }
        let conference = if may_move {
            let Some(conference) = self.ask_target_conference(cmd, true).await? else { return Ok(AfterAction::Redisplay) };
            conference
        } else { source_conf };
        let area = if conference == source_conf { self.session.current_message_area } else {
            let Some(area) = self.ask_target_area(conference).await? else { return Ok(AfterAction::Redisplay) };
            area
        };
        let Some(target) = self.read_action_target(conference, area).await? else { return Ok(AfterAction::Redisplay) };
        let old_to = header.to().map(ToString::to_string).unwrap_or_default();
        // Recipient policy belongs to the target conference; restore context on
        // both success and error, without joining or acquiring its security bonus.
        let saved = self.session.current_conference.clone();
        let target_conf = self.get_board().await.conferences[conference as usize].clone();
        self.session.current_conference = target_conf;
        self.session.current_conference_number = conference;
        let recipient = self.get_message_recipient(IceText::MessageTo, old_to, false).await;
        self.session.current_conference = saved;
        self.session.current_conference_number = source_conf;
        let Some(recipient) = recipient? else { return Ok(AfterAction::Redisplay) };
        if recipient.eq_ignore_ascii_case("@LIST@") || self.session.request_logoff { return Ok(AfterAction::Redisplay); }
        let draft = transfer_draft(original, same_message_base(base.path(), &target));
        let mut header = draft.header().clone();
        header.set_to(BString::from(recipient.clone()));
        header.attributes &= !(attributes::MSG_READ | attributes::MSG_SENT);
        header.date_received = 0;
        header.times_read = 0;
        header.sub_fields.retain(|field| field.field_type() != SubfieldType::AddressD
            && !(field.field_type() == SubfieldType::FTSKludge && field.content().starts_with(b"ICYBOARD-FORWARD: ")));
        if recipient.contains('@') { header.sub_fields.push(MessageSubfield::new(SubfieldType::AddressD, BString::from(recipient))); }
        header.sub_fields.push(MessageSubfield::new(SubfieldType::FTSKludge,
            BString::from(format!("ICYBOARD-FORWARD: {source_conf} {}", self.session.get_username_or_alias()))));
        let draft = JamMessage::from_stored(header, draft.text().clone());
        let mut attachments = match self.copy_action_attachments(&draft, conference, area).await {
            Ok(attachments) => attachments,
            Err(error) => {
                log::error!("Could not forward attachments for message {number}: {error}");
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                return Ok(AfterAction::Redisplay);
            }
        };
        if let Err(error) = self.send_action_message(conference, area, &target, draft, IceText::MessageCopied, &mut attachments).await {
            log::error!("Could not forward message {number}; original retained: {error}");
            self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
        }
        Ok(AfterAction::Redisplay)
    }

    /// Copy enclosure files, never rename/remove the originals. Preserve their
    /// stored names because EnclFile/alias metadata is part of the message.
    /// Roll back newly copied files unless a message actually references them.
    async fn copy_action_attachments(&mut self, message: &JamMessage, conference: u16, area: usize) -> Res<ActionAttachments> {
        let mut copies = ActionAttachments::default();
        let fields: Vec<_> = message.header().sub_fields.iter()
            .filter(|field| matches!(field.field_type(), SubfieldType::EnclFile | SubfieldType::EnclFwAlias)).collect();
        if fields.is_empty() { return Ok(copies); }
        let source = self.session.current_conference.attachment_location.clone();
        let target = self.get_board().await.conferences.get(conference as usize).cloned()
            .ok_or_else(|| std::io::Error::other("Invalid attachment conference"))?;
        if source.as_os_str().is_empty() || target.attachment_location.as_os_str().is_empty()
            || !target.sec_attachments.session_can_access(&self.session)
            || !target.areas.as_ref().and_then(|areas| areas.get(area))
                .is_some_and(|area| area.req_level_to_save_attach.session_can_access(&self.session)) {
            return Err(std::io::Error::other("Attachments are not allowed in the destination").into());
        }
        let source = self.resolve_path(&source).canonicalize()?;
        let target = self.resolve_path(&target.attachment_location).canonicalize()?;
        for field in fields {
            copies.copy(&source, &target, field)?;
        }
        Ok(copies)
    }

    /// send_message can fail after append (statistics or terminal output). For
    /// new enclosure copies, adopt them at the append, not at its UI result.
    async fn send_action_message(&mut self, conference: u16, area: usize, target: &Path, message: JamMessage, text: IceText,
        attachments: &mut ActionAttachments) -> Res<()> {
        if attachments.files.is_empty() {
            return self.send_message(conference as i32, area as i32, message, text).await;
        }
        let mut base = if target.with_extension("jhr").exists() {
            JamMessageBase::open(target)?
        } else {
            JamMessageBase::create(target)?
        };
        let number = base.write_message(&message)?;
        attachments.commit();
        if let Some(user) = &mut self.session.current_user { user.stats.messages_left += 1; }
        self.get_board().await.statistics.add_message();
        self.get_board().await.save_statistics()?;
        self.display_text(text, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &number.to_string()).await?;
        self.new_line().await?;
        Ok(())
    }

    async fn read_attachment(&mut self, action: MsgFunc, base: &mut JamMessageBase, number: u32) -> Res<AfterAction> {
        let Some(header) = self.read_action_header(base, number).await? else { return Ok(AfterAction::Prompt) };
        let sec = self.session.user_command_level.cmd_d.clone();
        if action == MsgFunc::FlagFile && !self.check_sec("FLAG", &sec).await? { return Ok(AfterAction::Prompt); }
        let fields: Vec<_> = header.sub_fields.iter()
            .filter(|field| matches!(field.field_type(), SubfieldType::EnclFile | SubfieldType::EnclFwAlias)).collect();
        if fields.is_empty() {
            if action == MsgFunc::FlagFile {
                if self.session.current_conference.directories.is_some() { self.flag_files_cmd(true).await?; }
            } else { self.view_file().await?; }
            return Ok(AfterAction::Prompt);
        }
        let location = self.session.current_conference.attachment_location.clone();
        if location.as_os_str().is_empty() || !self.session.current_conference.sec_attachments.session_can_access(&self.session) {
            self.display_text(IceText::AttachNotAllOWed, display_flags::NEWLINE).await?;
            return Ok(AfterAction::Prompt);
        }
        let root = self.resolve_path(&location);
        for field in fields {
            let (path, alias) = match attachment_path(&root, field) {
                Ok(attachment) => attachment,
                Err(error) => {
                    log::error!("Rejected message attachment: {error}");
                    self.display_text(IceText::ErrorViewingFile, display_flags::NEWLINE).await?;
                    continue;
                }
            };
            if action == MsgFunc::FlagFile { self.add_flagged_file(path, false, true).await?; }
            else {
                // Isolate view's pattern-based lookup to a private directory with
                // exactly one regular file. No global rename or wildcard escape.
                let temp = tempfile::tempdir()?;
                std::fs::copy(&path, temp.path().join(&alias))?;
                let saved_tokens = std::mem::take(&mut self.session.tokens);
                let saved_list = self.session.disp_options.in_file_list.replace(temp.path().to_path_buf());
                self.session.tokens.push_back(alias);
                let result = self.view_file().await;
                self.session.tokens = saved_tokens;
                self.session.disp_options.in_file_list = saved_list;
                result?;
            }
        }
        Ok(AfterAction::Prompt)
    }

    /// Native X exports a raw JAM header plus body, using an isolated temporary
    /// EXPORT.MSG and the existing transfer UI. There is no configured DOS
    /// EXPORT.BAT hook in this engine; never execute a caller-supplied filename.
    async fn export_read_message(&mut self, base: &mut JamMessageBase, number: u32) -> Res<AfterAction> {
        let sec = self.get_board().await.config.sysop_command_level.read_all_mail.clone();
        if !self.check_sec("X", &sec).await? { return Ok(AfterAction::Redisplay); }
        let Some(message) = self.read_action_message(base, number).await? else { return Ok(AfterAction::Redisplay) };
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("EXPORT.MSG");
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        message.header().write(&mut file)?;
        file.write_all(message.text())?;
        file.sync_all()?;
        drop(file);
        let saved_files = std::mem::replace(&mut self.session.flagged_files, vec![path]);
        let saved_tokens = std::mem::take(&mut self.session.tokens);
        let saved_list = self.session.disp_options.in_file_list.take();
        let result = self.download(false).await;
        self.session.flagged_files = saved_files;
        self.session.tokens = saved_tokens;
        self.session.disp_options.in_file_list = saved_list;
        result?;
        Ok(AfterAction::Redisplay)
    }

    pub(super) async fn run_read_action(&mut self, cmd: &ReadCommand, message_base: &mut JamMessageBase, number: u32) -> Res<AfterAction> {
        match cmd.func {
            MsgFunc::Kill => {
                self.new_line().await?;
                let sec = self.session.user_command_level.cmd_k.clone();
                if self.check_sec("K", &sec).await? {
                    self.try_to_kill_message(message_base, number).await?;
                }
                Ok(AfterAction::Next)
            }
            MsgFunc::Protect | MsgFunc::Unprotect => {
                self.new_line().await?;
                let protect = cmd.func == MsgFunc::Protect;
                let sec = self.get_board().await.config.sysop_command_level.protect_unprotect_messages.clone();
                if self.check_sec(if protect { "P" } else { "U" }, &sec).await? {
                    let Some(mut header) = self.read_action_header(message_base, number).await? else { return Ok(AfterAction::Redisplay) };
                    let original = header.clone();
                    clear_password(&mut header);
                    if protect { header.attributes |= attributes::MSG_PRIVATE; }
                    else { header.attributes &= !attributes::MSG_PRIVATE; }
                    self.write_header(message_base, number, &original, &header).await?;
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::Move | MsgFunc::Copy => {
                self.new_line().await?;
                let moving = cmd.func == MsgFunc::Move;
                let sec = self.get_board().await.config.sysop_command_level.copy_move_messages.clone();
                if !self.check_sec(if moving { "MOVE" } else { "COPY" }, &sec).await? {
                    return Ok(AfterAction::Prompt);
                }
                let Some(conference) = self.ask_target_conference(cmd, moving).await? else {
                    return Ok(AfterAction::Prompt);
                };
                let Some(area) = self.ask_target_area(conference).await? else {
                    return Ok(AfterAction::Prompt);
                };
                if !self.copy_message_to_conference(message_base, number, conference, area, moving).await? {
                    return Ok(AfterAction::Prompt);
                }
                if moving {
                    return Ok(AfterAction::Next);
                }
                Ok(AfterAction::Redisplay)
            }
            // J remembers nothing here: PCBoard leaves the reader and lets the
            // main prompt run the join with the tokens that follow.
            MsgFunc::Join | MsgFunc::JumpOut => Ok(AfterAction::Quit),
            MsgFunc::Skip => {
                // SKIPEND drags the pointer to the end before leaving, so the
                // conference counts as read, and says where it left it.
                let high = message_base.highest_message_number();
                self.session.last_msg_read = high;
                self.session.highest_msg_read = self.session.highest_msg_read.max(high);
                self.store_last_read(message_base, high)?;
                self.session.op_text = high.to_string();
                self.display_text(IceText::LastMessageReadSetTo, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                Ok(AfterAction::Quit)
            }
            MsgFunc::EnterMessage => {
                let sec = self.session.user_command_level.cmd_e.clone();
                if self.check_sec("E", &sec).await? {
                    self.enter_message().await?;
                }
                Ok(AfterAction::Quit)
            }
            MsgFunc::Reply | MsgFunc::ReplyOther => {
                self.new_line().await?;
                let sec = self.session.user_command_level.cmd_e.clone();
                if self.check_sec("REPLY", &sec).await? {
                    let source = message_base.path().to_path_buf();
                    let email_path = self.email_msgbase_path().await;
                    let email = same_message_base(&source, &email_path);
                    let result = self.reply_from_base(&source, number, cmd.func == MsgFunc::ReplyOther, email).await?;
                    return Ok(Self::after_message_save(result));
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::QuickScan => {
                self.quick_message_scan().await?;
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::SelectConference | MsgFunc::DeselectConference => {
                self.select_conferences(SelectMode::SelectCmd).await?;
                Ok(AfterAction::Next)
            }
            MsgFunc::Chat => {
                let sec = self.session.user_command_level.cmd_chat.clone();
                if self.check_sec("CHAT", &sec).await? {
                    self.group_chat_command().await?;
                    // The message is redrawn over the top, so PCBoard waits here.
                    self.press_enter().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::Who => {
                let sec = self.session.user_command_level.cmd_who.clone();
                if self.check_sec("WHO", &sec).await? {
                    self.who_display_nodes().await?;
                    self.press_enter().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::EditMessage => self.edit_read_message(message_base, number).await,
            MsgFunc::Forward => self.forward_read_message(cmd, message_base, number).await,
            MsgFunc::Export => self.export_read_message(message_base, number).await,
            MsgFunc::FlagFile | MsgFunc::ViewFile => self.read_attachment(cmd.func, message_base, number).await,
            // PCBoard answers the sender or recipient of the message in front of
            // the reader by handing the name to user maintenance.
            MsgFunc::FindTo | MsgFunc::FindFrom => {
                self.new_line().await?;
                let sec = self.get_board().await.config.sysop_command_level.sec_7_user_maint.clone();
                if self.check_sec("F", &sec).await? {
                    let Ok(header) = message_base.read_header(number) else {
                        return Ok(AfterAction::Redisplay);
                    };
                    let name = if cmd.func == MsgFunc::FindTo { header.to() } else { header.from() };
                    if let Some(name) = name {
                        self.session.tokens.push_front(name.to_string());
                    }
                    self.user_maintenance().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            _ => Ok(AfterAction::NotHandled),
        }
    }

    /// Moves this user's last-read pointer for the base in front of the reader.
    fn store_last_read(&mut self, message_base: &mut JamMessageBase, number: u32) -> Res<()> {
        unsafe {
            let crc = JamMessageBase::crc(&BString::new(self.session.user_name.as_mut_vec().clone()));
            let user_id = self.session.cur_user_id as u32;
            let mut last_read = message_base
                .find_last_read(crc, user_id)?
                .unwrap_or(message_base.create_last_read(crc, user_id)?);
            last_read.last_read_msg = number;
            last_read.high_read_msg = last_read.high_read_msg.max(number);
            message_base.write_last_read(&last_read)?;
        }
        Ok(())
    }

    /// `PCBoard` asks for the conference only when the command line did not carry one.
    async fn ask_target_conference(&mut self, cmd: &ReadCommand, moving: bool) -> Res<Option<u16>> {
        let num_conferences = self.get_board().await.conferences.len();
        if let Some(conference) = cmd.move_conf {
            return Ok(if (conference as usize) < num_conferences { Some(conference) } else { None });
        }
        let prompt = if moving {
            IceText::MovedMessageToConference
        } else {
            IceText::CopyMessageToConference
        };
        let answer = self
            .input_field(prompt, 5, &MASK_NUM, "hlpendr", None, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        if answer.is_empty() {
            return Ok(None);
        }
        let Ok(conference) = answer.parse::<u16>() else {
            return Ok(None);
        };
        Ok(if (conference as usize) < num_conferences { Some(conference) } else { None })
    }

    /// A conference is split into message areas in `icy_board`, which `PCBoard` has
    /// no notion of. A board shaped the way `PCBoard` expects has one area per
    /// conference, so nothing extra is asked and a PPE stuffing the keyboard
    /// still sees the prompts it was written for.
    async fn ask_target_area(&mut self, conference: u16) -> Res<Option<usize>> {
        let target = self.get_board().await.conferences.get(conference as usize).cloned();
        let Some(target) = target else { return Ok(None) };
        if !self.action_conference_access(conference, &target) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
            return Ok(None);
        }
        let areas = target.areas.clone().unwrap_or_default();
        if areas.is_empty() { return Ok(None); }
        if areas.len() == 1 {
            return Ok(Some(0));
        }
        for (i, area) in areas.iter().enumerate() {
            if !area.req_level_to_list.session_can_access(&self.session) { continue; }
            self.print(TerminalTarget::Both, &format!("{:>3}) {}", i + 1, area.name)).await?;
            self.new_line().await?;
        }
        let answer = self
            .input_field(
                IceText::JoinAreaNumber,
                5,
                &MASK_NUM,
                "",
                Some("1".to_string()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
            )
            .await?;
        if answer.is_empty() {
            return Ok(Some(0));
        }
        match answer.parse::<usize>() {
            Ok(number) if number >= 1 && number <= areas.len() => Ok(Some(number - 1)),
            _ => {
                self.session.op_text = answer;
                self.display_text(IceText::InvalidAreaNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                Ok(None)
            }
        }
    }

    async fn copy_message_to_conference(&mut self, message_base: &mut JamMessageBase, number: u32, conference: u16, area: usize, moving: bool) -> Res<bool> {
        let Some(target) = self.read_action_target(conference, area).await? else { return Ok(false) };
        let Some(original) = self.read_action_message(message_base, number).await? else { return Ok(false) };
        let same_base = same_message_base(message_base.path(), &target);
        let msg = transfer_draft(JamMessage::from_stored(original.header().clone(), original.text().clone()), same_base);
        let mut attachments = match self.copy_action_attachments(&msg, conference, area).await {
            Ok(attachments) => attachments,
            Err(error) => {
                log::error!("Could not copy attachments for message {number}; source retained: {error}");
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                return Ok(false);
            }
        };
        let text = if moving { IceText::MessageMoved } else { IceText::MessageCopied };
        let result = finish_transfer(self.send_action_message(conference, area, &target, msg, text, &mut attachments), &target, moving,
            || delete_unchanged_message(message_base, number, &original)).await;
        match result {
            Ok(()) => Ok(true),
            Err(error) => {
                match error {
                    TransferFailure::Destination(error) => log::error!("Message {number} destination failed; source retained: {error}"),
                    TransferFailure::Source(error) => log::error!("Message {number} copied but source deletion failed; both copies retained: {error}"),
                }
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                Ok(false)
            }
        }
    }

    /// E inside the read loop: change one field of the header
    /// in front of the reader. The option letter and the follow-up question are
    /// what a stuffing PPE counts on, so they come in `PCBoard`'s order.
    pub(super) async fn edit_header(&mut self, message_base: &mut JamMessageBase, number: u32) -> Res<()> {
        self.new_line().await?;
        let sec = self.session.user_command_level.cmd_e.clone();
        if !self.check_sec("E", &sec).await? {
            return Ok(());
        }
        let Some(mut header) = self.read_action_header(message_base, number).await? else { return Ok(()) };
        let original = header.clone();

        let from = header.from().map(std::string::ToString::to_string).unwrap_or_default();
        let edit_all = self.get_board().await.config.sysop_command_level.edit_any_message.clone();
        let own = from.eq_ignore_ascii_case(&self.session.user_name)
            || (!self.session.alias_name.is_empty() && from.eq_ignore_ascii_case(&self.session.alias_name));
        if !own && !edit_all.session_can_access(&self.session) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }

        let to = header.to().map(std::string::ToString::to_string).unwrap_or_default();
        let subject = header.subject().map(std::string::ToString::to_string).unwrap_or_default();
        self.display_text(IceText::To, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &to).await?;
        self.display_text(IceText::From, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &from).await?;
        self.display_text(IceText::Subject, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &subject).await?;

        let echo_mail = self.session.current_conference.echo_mail_in_conference;
        let option = self
            .input_field(
                if echo_mail { IceText::EditHeaderEcho } else { IceText::EditHeader },
                1,
                if echo_mail { "EFNPRST" } else { "FNPRST" },
                "",
                None,
                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;

        let (old, len) = match option.as_str() {
            "E" => {
                header.attributes ^= attributes::MSG_TYPEECHO;
                return self.write_header(message_base, number, &original, &header).await;
            }
            "R" => {
                let sec = self.get_board().await.config.sysop_command_level.edit_message_headers.clone();
                if !self.check_sec("R", &sec).await? {
                    return Ok(());
                }
                if header.attributes & attributes::MSG_READ == 0 {
                    return Ok(());
                }
                header.attributes &= !attributes::MSG_READ;
                return self.write_header(message_base, number, &original, &header).await;
            }
            "P" => {
                let sec = self.get_board().await.config.sysop_command_level.protect_unprotect_messages.clone();
                if !self.check_sec("P", &sec).await? {
                    return Ok(());
                }
                let current = if header.needs_password() {
                    if requires_read_password(&header, false) { "G" } else { "S" }
                } else if header.attributes & attributes::MSG_PRIVATE != 0 {
                    "R"
                } else {
                    "N"
                };
                let answer = self
                    .input_field(
                        IceText::MessageSecurity,
                        1,
                        "NRSG",
                        "hlpsec",
                        Some(current.to_string()),
                        display_flags::UPCASE | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                match answer.as_str() {
                    "N" => {
                        header.attributes &= !attributes::MSG_PRIVATE;
                        clear_password(&mut header);
                    }
                    "R" => {
                        if self.session.current_conference.disallow_private_msgs {
                            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
                            return Ok(());
                        }
                        header.attributes |= attributes::MSG_PRIVATE;
                        clear_password(&mut header);
                    }
                    "S" | "G" => {
                        let password = self
                            .input_field(
                                IceText::SecurityPassword,
                                12,
                                &MASK_ASCII,
                                "hlpe",
                                None,
                                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
                            )
                            .await?;
                        if password.is_empty() || self.session.request_logoff { return Ok(()); }
                        header.attributes &= !attributes::MSG_PRIVATE;
                        header.password_crc = JamMessageBase::crc(&BString::from(password.as_str()));
                        set_security_kind(&mut header, answer == "S");
                    }
                    _ => return Ok(()),
                }
                return self.write_header(message_base, number, &original, &header).await;
            }
            "N" => (header.reply_to.to_string(), 9),
            "T" => (to.clone(), 25),
            "F" => {
                let sec = self.get_board().await.config.sysop_command_level.edit_message_headers.clone();
                if !self.check_sec("F", &sec).await? {
                    return Ok(());
                }
                (from.clone(), 25)
            }
            "S" => (subject.clone(), 60),
            _ => return Ok(()),
        };

        let answer = self
            .input_field(
                IceText::NewInfo,
                len,
                &MASK_ASCII,
                "",
                Some(old.clone()),
                display_flags::FIELDLEN | display_flags::HIGHASCII | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        let answer = answer.trim().to_string();
        if answer.is_empty() || answer == old {
            return Ok(());
        }

        match option.as_str() {
            "N" => match answer.parse::<u32>() {
                Ok(reply_to) => header.reply_to = reply_to,
                Err(_) => return Ok(()),
            },
            "T" => replace_sub_field(&mut header, SubfieldType::RecvName, &answer),
            "F" => {
                let answer = answer.to_ascii_uppercase();
                if answer.contains("@USER@") {
                    self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
                replace_sub_field(&mut header, SubfieldType::SenderName, &answer);
            }
            "S" => replace_sub_field(&mut header, SubfieldType::Subject, &answer),
            _ => return Ok(()),
        }
        self.write_header(message_base, number, &original, &header).await
    }

    async fn write_header(&mut self, message_base: &mut JamMessageBase, number: u32, original: &JamMessageHeader, header: &JamMessageHeader) -> Res<()> {
        if let Err(err) = replace_header(message_base, number, original, header) {
            log::error!("Error writing the header of message {number}: {err}");
            self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
        }
        Ok(())
    }

    /// R's SET command: move this conference's last-read pointer.
    pub(super) async fn set_last_message_read(&mut self, cmd: &ReadCommand, message_base: &mut JamMessageBase) -> Res<()> {
        let low = message_base.lowest_message_number();
        let high = message_base.highest_message_number();

        let number = if let Some(number) = cmd.new_last_read {
            number
        } else {
            self.session.op_text = format!("{low}-{high}");
            let answer = self
                .input_field(
                    IceText::SetLastMessageReadPointer,
                    9,
                    &MASK_NUM,
                    "",
                    Some(self.session.last_msg_read.to_string()),
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
                )
                .await?;
            if answer.is_empty() {
                return Ok(());
            }
            match answer.parse::<i64>() {
                Ok(number) => number,
                Err(_) => return Ok(()),
            }
        };

        let number = number.clamp(0, high as i64) as u32;
        unsafe {
            let crc = JamMessageBase::crc(&BString::new(self.session.user_name.as_mut_vec().clone()));
            let mut last_read = message_base
                .find_last_read(crc, self.session.cur_user_id as u32)?
                .unwrap_or(message_base.create_last_read(crc, self.session.cur_user_id as u32)?);
            last_read.last_read_msg = number;
            message_base.write_last_read(&last_read)?;
        }
        self.session.last_msg_read = number;

        self.session.op_text = number.to_string();
        self.display_text(IceText::LastMessageReadSetTo, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        Ok(())
    }
}
