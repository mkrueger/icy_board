use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::user_base::ConferenceFlags;
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

/// The column widths of the header this list sits under.
const NAME_WIDTH: usize = 25;
const CITY_WIDTH: usize = 24;

/// PCBoard's FidoNet placeholder account, which is not a caller.
const FIDO_ACCOUNT: &str = "~FIDO~";

impl IcyBoardState {
    pub async fn show_user_list_cmd(&mut self) -> Res<()> {
        self.new_line().await?;

        // Every token belongs to the search text, not just the first.
        let mut text = String::new();
        while let Some(token) = self.session.tokens.pop_front() {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&token);
        }
        if text.is_empty() {
            text = self
                .input_field(
                    IceText::UserScan,
                    40,
                    MASK_COMMAND,
                    CommandType::UserList.get_help(),
                    None,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?;
        }
        let text = text.trim().to_string();

        // The same search language the file and message scans use.
        if !text.is_empty() && !self.search_init(text.clone(), false) {
            self.display_text(IceText::PunctuationError, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }

        self.session.disp_options.no_change();
        self.display_text(IceText::UsersHeader, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK)
            .await?;
        self.display_text(IceText::UserScanLine, display_flags::NOTBLANK).await?;
        self.reset_color(TerminalTarget::Both).await?;
        self.new_line().await?;

        let conference = self.session.current_conference_number as usize;
        let is_sysop = self.session.is_sysop;
        let pattern = self.session.search_pattern.clone();

        let mut lines = Vec::new();
        for (record, user) in self.get_board().await.users.iter().enumerate() {
            // Record one is the sysop's own, and only the sysop sees it.
            if record == 0 && !is_sysop {
                continue;
            }
            if user.security_level == 0 || user.get_name().eq_ignore_ascii_case(FIDO_ACCOUNT) {
                continue;
            }
            if !registered_in(user, conference) {
                continue;
            }
            // The search covers the name and the location, as it did in the original.
            if let Some(pattern) = &pattern {
                if !pattern.is_match(&format!("{} {}", user.get_name(), user.city_or_state)) {
                    continue;
                }
            }
            lines.push(format!(
                "{:<name_width$} {:<city_width$}  {:<8}  {:<5}",
                truncate(user.get_name(), NAME_WIDTH),
                truncate(&user.city_or_state, CITY_WIDTH),
                self.format_date(user.stats.last_on),
                self.format_time(user.stats.last_on),
                name_width = NAME_WIDTH,
                city_width = CITY_WIDTH,
            ));
        }

        for line in lines {
            self.println(TerminalTarget::Both, line.trim_end()).await?;
            if self.session.disp_options.abort_printout {
                break;
            }
        }
        self.stop_search();
        Ok(())
    }
}

/// Conference zero is open to everyone; the rest need a registration flag.
fn registered_in(user: &crate::icy_board::user_base::User, conference: usize) -> bool {
    conference == 0
        || user
            .conference_flags
            .get(&conference)
            .is_some_and(|flags| flags.contains(ConferenceFlags::Registered))
}

fn truncate(text: &str, len: usize) -> &str {
    match text.char_indices().nth(len) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}
