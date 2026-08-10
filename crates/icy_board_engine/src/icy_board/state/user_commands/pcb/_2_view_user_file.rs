use crate::icy_board::commands::CommandType;
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Sysop command 2 - the whole user file, not the public user list.
    pub async fn view_user_file(&mut self) -> Res<()> {
        let answer = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::ViewPrintUsers,
                1,
                "VP",
                CommandType::ViewUserFile.get_help(),
                None,
                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?
        };

        match answer.to_ascii_uppercase().as_str() {
            "V" => {}
            "P" => {
                // No printer here, so the listing runs without stopping instead.
                self.session.disp_options.force_non_stop();
            }
            _ => return Ok(()),
        }

        self.new_line().await?;
        self.session.disp_options.force_count_lines();

        let users = self.get_board().await.users.clone();
        for u in users.iter() {
            let line = format!(
                "{:<25} {:<20} {:>5} {} {}",
                u.get_name(),
                u.city_or_state,
                u.security_level,
                self.format_date(u.stats.last_on),
                self.format_time(u.stats.last_on)
            );
            self.print(TerminalTarget::Both, &line).await?;
            self.new_line().await?;
            if self.session.disp_options.abort_printout {
                break;
            }
        }

        self.display_text(IceText::UsersFileViewed, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        Ok(())
    }
}
