use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::functions::pwd_flags;
use crate::icy_board::state::user_commands::mods::messagereader::message_security::{may_read_header, requires_read_password};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use jamjam::jam::{JamMessage, JamMessageBase, msg_header::JamMessageHeader};

#[cfg(test)]
#[path = "k_delete_message_tests.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq)]
enum KillPermission {
    Denied,
    Allowed,
    SenderPassword,
}

struct KillAccess<'a> {
    user: &'a str,
    alias: &'a str,
    command: bool,
    read_all: bool,
}

impl KillAccess<'_> {
    fn permission(&self, header: &JamMessageHeader) -> KillPermission {
        if !self.command || !may_read_header(header, self.user, self.alias, self.read_all) {
            return KillPermission::Denied;
        }
        // JAM comments currently carry only MSG_PRIVATE, with no distinct CMNT
        // marker. Do not grant read_all_comments for arbitrary private mail.
        if self.read_all {
            return KillPermission::Allowed;
        }
        let matches = |name: Option<&bstr::BString>| {
            name.is_some_and(|name| {
                let name = name.to_string();
                (!self.user.is_empty() && name.trim().eq_ignore_ascii_case(self.user))
                    || (!self.alias.is_empty() && name.trim().eq_ignore_ascii_case(self.alias))
            })
        };
        let sender_password = header.needs_password() && !requires_read_password(header, false);
        if !sender_password && matches(header.to()) && !header.to().is_some_and(|to| to.to_string().trim_start().starts_with('@')) {
            return KillPermission::Allowed;
        }
        if matches(header.from()) {
            if header.needs_password() {
                KillPermission::SenderPassword
            } else {
                KillPermission::Allowed
            }
        } else {
            KillPermission::Denied
        }
    }
}

struct KillSnapshot {
    message: JamMessage,
    generation: u32,
}

/// Own header and body from the same locked generation, releasing the lock
/// before any password input. Unauthorized callers never need the body.
fn kill_snapshot(base: &mut JamMessageBase, number: u32, access: &KillAccess<'_>) -> jamjam::Result<Option<KillSnapshot>> {
    base.read_transaction(|base| {
        let header = base.read_header(number)?;
        if access.permission(&header) == KillPermission::Denied {
            return Ok(None);
        }
        let body = base.read_message_text(&header)?;
        Ok(Some(KillSnapshot {
            message: JamMessage::from_stored(header, body),
            generation: base.mod_counter(),
        }))
    })
}

/// A number or MsgID CRC is not an identity: pack can reuse both. Compare all
/// header fields (including offsets/security) and the body under the same
/// exclusive transaction as deletion. The base generation also rejects an
/// identical duplicate renumbered into this slot; conservatively retry even
/// after unrelated base writes. Same-length in-place body edits are stale too.
fn kill_unchanged(base: &mut JamMessageBase, number: u32, snapshot: &KillSnapshot, access: &KillAccess<'_>, password_valid: bool) -> jamjam::Result<bool> {
    base.transaction(|base| {
        let fresh = base.read_header(number)?;
        if base.mod_counter() != snapshot.generation {
            return Ok(false);
        }
        let mut expected = Vec::new();
        let mut actual = Vec::new();
        snapshot.message.header().write(&mut expected)?;
        fresh.write(&mut actual)?;
        if actual != expected || base.read_message_text(&fresh)? != *snapshot.message.text() {
            return Ok(false);
        }
        match access.permission(&fresh) {
            KillPermission::Denied => return Ok(false),
            KillPermission::SenderPassword if !password_valid => return Ok(false),
            _ => {}
        }
        base.delete_message(number)?;
        Ok(true)
    })
}

impl IcyBoardState {
    pub async fn delete_message(&mut self) -> Res<()> {
        let Some(area) = self
            .session
            .current_conference
            .areas
            .as_ref()
            .and_then(|areas| areas.get(self.session.current_message_area))
            .filter(|area| !area.path.as_os_str().is_empty())
        else {
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(());
        };
        let message_base_file = self.resolve_path(&area.path);
        let read_security = area.req_level_to_list.clone();
        if !self.check_sec("K", &read_security).await? {
            return Ok(());
        }

        match JamMessageBase::open(&message_base_file) {
            Ok(mut message_base) => {
                let msg = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.session.op_text = format!("{}-{}", message_base.lowest_message_number(), message_base.highest_message_number());

                    self.input_field(
                        IceText::MessageNumberToKill,
                        40,
                        MASK_COMMAND,
                        CommandType::DeleteMessage.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };

                if let Ok(number) = msg.parse::<u32>() {
                    self.try_to_kill_message(&mut message_base, number).await?;
                }
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {err}");
                // A kill command must not create or replace an unreadable base.
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                Ok(())
            }
        }
    }

    pub(crate) async fn try_to_kill_message(&mut self, message_base: &mut JamMessageBase, number: u32) -> Res<()> {
        let read_all = self
            .get_board()
            .await
            .config
            .sysop_command_level
            .read_all_mail
            .session_can_access(&self.session);
        let access = KillAccess {
            user: &self.session.user_name,
            alias: &self.session.alias_name,
            command: self.session.user_command_level.cmd_k.session_can_access(&self.session),
            read_all,
        };
        let snapshot = match kill_snapshot(message_base, number, &access) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return self.kill_message_result(IceText::YouCanNotKillMessage, number).await,
            Err(err) => {
                log::error!("Error reading message {number} ({})/ {err}", message_base.path().display());
                return self.kill_message_result(IceText::NoSuchMessageNumber, number).await;
            }
        };
        self.try_to_kill_reply_source(message_base, snapshot.message, snapshot.generation).await.map(|_| ())
    }

    /// Use an already-owned source, never a newly selected message at its number.
    /// `generation` includes only the caller's known writes since the snapshot.
    pub(super) async fn try_to_kill_reply_source(&mut self, message_base: &mut JamMessageBase, original: JamMessage, generation: u32) -> Res<bool> {
        let number = original.header().message_number;
        let snapshot = KillSnapshot { message: original, generation };
        let read_all = self.get_board().await.config.sysop_command_level.read_all_mail.session_can_access(&self.session);
        let access = KillAccess {
            user: &self.session.user_name,
            alias: &self.session.alias_name,
            command: self.session.user_command_level.cmd_k.session_can_access(&self.session),
            read_all,
        };
        if access.permission(snapshot.message.header()) == KillPermission::Denied {
            self.kill_message_result(IceText::YouCanNotKillMessage, number).await?;
            return Ok(false);
        }
        let needs_password = access.permission(snapshot.message.header()) == KillPermission::SenderPassword;
        let password_valid = needs_password
            && self
                .check_password(IceText::YourPassword, pwd_flags::SHOW_WRONG_PWD_MSG, |pwd| {
                    snapshot.message.header().is_password_valid(pwd)
                })
                .await?;
        if self.session.request_logoff {
            return Ok(false);
        }
        if needs_password && !password_valid {
            self.kill_message_result(IceText::YouCanNotKillMessage, number).await?;
            return Ok(false);
        }

        // Refresh session authorization after input, without holding a JAM lock.
        let read_all = self
            .get_board()
            .await
            .config
            .sysop_command_level
            .read_all_mail
            .session_can_access(&self.session);
        let access = KillAccess {
            user: &self.session.user_name,
            alias: &self.session.alias_name,
            command: self.session.user_command_level.cmd_k.session_can_access(&self.session),
            read_all,
        };
        let mut killed = false;
        let text = match kill_unchanged(message_base, number, &snapshot, &access, password_valid) {
            Ok(true) => {
                killed = true;
                log::info!("Deleted message {number} ({})", message_base.path().display());
                IceText::MessageKilled
            }
            Ok(false) => IceText::YouCanNotKillMessage,
            Err(err) => {
                log::error!("Error deleting message {number} ({})/ {err}", message_base.path().display());
                IceText::NoSuchMessageNumber
            }
        };
        self.kill_message_result(text, number).await?;
        Ok(killed)
    }

    async fn kill_message_result(&mut self, text: IceText, number: u32) -> Res<()> {
        self.session.op_text = number.to_string();
        self.display_text(text, display_flags::DEFAULT).await?;
        self.print(TerminalTarget::Both, &number.to_string()).await?;
        self.new_line().await?;
        self.new_line().await
    }
}
