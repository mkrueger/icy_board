use jamjam::jam::{JamMessageBase, pack::PackOptions};

use crate::icy_board::commands::CommandType;
use crate::{Res, datetime::IcbDate, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::functions::{MASK_NUM, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Sysop command 3 - PCBoard shelled out to PCBPack for the current conference here.
    pub async fn pack_message_base(&mut self) -> Res<()> {
        let Some(areas) = self.session.current_conference.areas.clone() else {
            return Ok(());
        };

        if let Some(token) = self.session.tokens.pop_front() {
            if !token.to_ascii_uppercase().starts_with(self.session.yes_char.to_ascii_uppercase()) {
                return Ok(());
            }
        } else if !self.ask_yes_no(IceText::PackTheMessageBase, false).await? {
            return Ok(());
        }

        let mut options = PackOptions {
            index_only: self.ask_yes_no(IceText::GenerateNewIndex, false).await?,
            ..Default::default()
        };

        if !options.index_only {
            let answer = self
                .input_field(
                    IceText::PurgeOlderThan,
                    6,
                    &MASK_NUM,
                    CommandType::PackMessageBase.get_help(),
                    None,
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            let purge_date = IcbDate::parse(answer.trim());
            // "010180" is the answer that switches the date criteria off.
            let unset = purge_date.is_empty() || (purge_date.year() == 1980 && purge_date.month() == 1 && purge_date.day() == 1);
            options.purge_before = (!unset).then(|| purge_date.to_utc_date_time());

            options.purge_received_private = self.ask_yes_no(IceText::PurgePrivateReceived, false).await?;

            if self.ask_yes_no(IceText::RenumberDuringPack, false).await? {
                let answer = self
                    .input_field(
                        IceText::NewLowMessageNumber,
                        8,
                        &MASK_NUM,
                        CommandType::PackMessageBase.get_help(),
                        Some("1".to_string()),
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                // A number of zero, or none at all, aborts the renumbering.
                options.renumber_from = answer.trim().parse::<u32>().ok().filter(|number| *number > 0);
            }
        }

        self.new_line().await?;
        let mut removed = 0;
        let mut failed = false;
        for area in areas.iter() {
            match JamMessageBase::open(&area.path) {
                Ok(mut message_base) => match message_base.pack(&options) {
                    Ok(report) => removed += report.removed,
                    Err(err) => {
                        log::error!("Error packing message base {}: {}", area.path.display(), err);
                        failed = true;
                    }
                },
                Err(err) => {
                    log::error!("Error opening message base {}: {}", area.path.display(), err);
                    failed = true;
                }
            }
            if self.session.disp_options.abort_printout {
                break;
            }
        }

        if failed {
            self.display_text(
                IceText::MessageBaseError,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }

        self.display_text(IceText::MessagesSuccessfullyPacked, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        if removed > 0 {
            self.print(TerminalTarget::Both, &format!("{removed} message(s) removed.")).await?;
            self.new_line().await?;
        }
        Ok(())
    }
}
