use chrono::Utc;

use crate::icy_board::commands::CommandType;
use crate::icy_board::user_maintenance::{self, UserSelection};
use crate::{Res, datetime::IcbDate, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::functions::{MASK_NUM, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Sysop command 8 - drops the user records the sysop no longer wants to carry.
    pub async fn pack_user_file(&mut self) -> Res<()> {
        // Packing renumbers every record, so it is only offered to the first one.
        if self.session.cur_user_id != 0 {
            self.display_text(IceText::NotRecNumberOne, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                .await?;
            return Ok(());
        }

        if self
            .node_state
            .lock()
            .await
            .iter()
            .enumerate()
            .any(|(node, state)| node != self.node && state.is_some())
        {
            self.print(TerminalTarget::Both, "Pack unavailable while another caller is online.").await?;
            self.new_line().await?;
            return Ok(());
        }

        if !self.ask_yes_no(IceText::PackTheUsersFile, false).await? {
            return Ok(());
        }

        let keep_locked_out = self.ask_yes_no(IceText::KeepLockedOut, true).await?;

        let purge_date = self
            .input_field(
                IceText::PurgeOlderThan,
                8,
                &MASK_NUM,
                CommandType::PackUserFile.get_help(),
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        let purge_date = IcbDate::parse(&purge_date);

        let keep_security = self
            .input_field(
                IceText::KeepSecurity,
                3,
                &MASK_NUM,
                CommandType::PackUserFile.get_help(),
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        let keep_security = keep_security.trim().parse::<u8>().ok();

        self.display_text(IceText::CheckingUserFile, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;

        let selection = UserSelection {
            delete_flagged: true,
            last_on_before: (!purge_date.is_empty()).then(|| purge_date.to_utc_date_time()),
            keep_security_at_least: keep_security,
            keep_locked_out,
            protect_first_record: true,
            ..Default::default()
        };

        let mut board = self.board.lock().await;
        let users_file = board.resolve_file(&board.config.paths.user_file);
        if let Err(err) = user_maintenance::create_backup(&users_file) {
            log::error!("Could not back up the user file before packing: {err}");
            drop(board);
            self.display_text(
                IceText::ErrorInUsersFile,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }

        let original = board.users.clone();
        let report = user_maintenance::pack(&mut board.users, &selection, Utc::now());
        let save_result = board.save_userbase();
        if save_result.is_err() {
            board.users = original;
        }
        drop(board);

        if let Err(err) = save_result {
            log::error!("Could not save the user file after packing: {err}");
            self.display_text(
                IceText::ErrorInUsersFile,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }

        for name in &report.names {
            self.print(TerminalTarget::Both, name).await?;
            self.new_line().await?;
            if self.session.disp_options.abort_printout {
                break;
            }
        }
        self.session.op_text = report.changed.to_string();
        self.display_text(IceText::UsersFilePacked, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        Ok(())
    }
}
