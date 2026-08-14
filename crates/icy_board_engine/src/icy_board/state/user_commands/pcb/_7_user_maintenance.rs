use crate::icy_board::commands::CommandType;
use crate::icy_board::user_base::User;
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::functions::{MASK_COMMAND, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Sysop command 7 - walks the user file record by record and edits the
    /// flags a sysop needs while a caller is waiting.
    pub async fn user_maintenance(&mut self) -> Res<()> {
        let mut record = self.session.cur_user_id.max(0) as usize;
        let mut forward = true;
        let mut show_record = true;

        loop {
            let count = self.board.lock().await.users.len();
            if count == 0 {
                return Ok(());
            }
            if record >= count {
                record = count - 1;
            }

            if show_record {
                self.display_record(record).await?;
            }
            show_record = true;

            let prompt = if self.session.expert_mode() {
                IceText::UsermodeExpertmode
            } else {
                IceText::UsermodeNoExpert
            };
            let answer = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.input_field(
                    prompt,
                    6,
                    MASK_COMMAND,
                    CommandType::UserMaintenance.get_help(),
                    None,
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?
            };

            let answer = answer.trim().to_ascii_uppercase();
            if let Ok(number) = answer.trim_end_matches(['+', '-']).parse::<usize>() {
                // Records are counted from one at the prompt, like the file listing.
                record = number.saturating_sub(1).min(count - 1);
                if answer.ends_with('-') {
                    forward = false;
                } else if answer.ends_with('+') {
                    forward = true;
                }
                continue;
            }

            match answer.as_str() {
                "" | "+" | "-" => {
                    if answer == "+" {
                        forward = true;
                    } else if answer == "-" {
                        forward = false;
                    }
                    if forward {
                        if record + 1 >= count {
                            return Ok(());
                        }
                        record += 1;
                    } else {
                        if record == 0 {
                            return Ok(());
                        }
                        record -= 1;
                    }
                }
                "D" => {
                    if record > 0 && self.ask_yes_no(IceText::DeleteRecord, false).await? {
                        let mut board = self.board.lock().await;
                        let user = &mut board.users[record];
                        user.flags.delete_flag = true;
                        user.security_level = 0;
                        user.exp_security_level = 0;
                        let result = board.save_userbase();
                        drop(board);
                        self.report_save(result).await?;
                    }
                }
                "U" => {
                    if record > 0 {
                        let mut board = self.board.lock().await;
                        board.users[record].flags.delete_flag = false;
                        let result = board.save_userbase();
                        drop(board);
                        self.report_save(result).await?;
                    }
                }
                "F" => {
                    let name = self
                        .input_field(
                            IceText::UserScan,
                            30,
                            MASK_COMMAND,
                            CommandType::UserMaintenance.get_help(),
                            None,
                            display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                    let found = {
                        let board = self.board.lock().await;
                        let needle = name.trim().to_lowercase();
                        board
                            .users
                            .iter()
                            .position(|user| user.get_name().to_lowercase().contains(&needle) || user.alias.to_lowercase().contains(&needle))
                    };
                    if let Some(index) = found {
                        record = index;
                    } else {
                        self.session.op_text = name;
                        self.display_text(IceText::NotInUsersFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                            .await?;
                        show_record = false;
                    }
                }
                "L" | "S" => {
                    self.view_user_file().await?;
                    show_record = false;
                }
                "Q" => return Ok(()),
                _ => show_record = false,
            }
        }
    }

    async fn report_save(&mut self, result: Res<()>) -> Res<()> {
        match result {
            Ok(()) => {
                self.display_text(IceText::UserRecordUpdated, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
            }
            Err(err) => {
                log::error!("Could not save the user file: {}", err);
                self.display_text(
                    IceText::ErrorInUsersFile,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn display_record(&mut self, record: usize) -> Res<()> {
        let user: User = {
            let board = self.board.lock().await;
            let Some(user) = board.users.get(record) else {
                return Ok(());
            };
            user.clone()
        };

        let lines = vec![
            format!("#{:<6} {}", record + 1, user.get_name()),
            format!("Alias      : {}", user.alias),
            format!("City       : {}", user.city_or_state),
            format!("Phone      : {} / {}", user.bus_data_phone, user.home_voice_phone),
            format!("Security   : {} (expired {})", user.security_level, user.exp_security_level),
            format!(
                "Expires    : {}",
                if user.expiration_date == chrono::DateTime::<chrono::Utc>::default() {
                    String::new()
                } else {
                    self.format_date(user.expiration_date)
                }
            ),
            format!(
                "Last on    : {} {}  calls {}",
                self.format_date(user.stats.last_on),
                self.format_time(user.stats.last_on),
                user.stats.num_times_on
            ),
            format!("Transfers  : {} up / {} down", user.stats.num_uploads, user.stats.num_downloads),
            format!(
                "Flags      : {}{}",
                if user.flags.delete_flag { "deleted " } else { "" },
                if user.flags.disabled_flag { "disabled" } else { "" }
            ),
            format!("Comment    : {}", user.user_comment),
        ];

        self.new_line().await?;
        for line in lines {
            self.print(TerminalTarget::Both, &line).await?;
            self.new_line().await?;
        }
        Ok(())
    }
}
