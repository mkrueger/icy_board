use crate::datetime::IcbTime;
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn page_sysop_command(&mut self) -> Res<()> {
        if !self.sysop_is_available().await {
            self.display_text(IceText::SysopUnAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return self.ask_comment_instead().await;
        }

        // A chat, or a page the user gave up on, ends the command. Only a page
        // that ran out of rings falls through to the comment.
        if self.page_sysop().await? {
            return Ok(());
        }
        self.new_line().await?;
        self.display_text(IceText::SysopUnAvailable, display_flags::LFBEFORE).await?;
        self.ask_comment_instead().await
    }

    /// The sysop has to have the page bell on and the clock has to be inside the
    /// page window.
    async fn sysop_is_available(&mut self) -> bool {
        let board = self.get_board().await;
        if !board.config.options.page_bell {
            return false;
        }
        let start = board.config.limits.sysop_start.to_pcboard_time();
        let stop = board.config.limits.sysop_stop.to_pcboard_time();
        drop(board);

        // An unset window means the page bell alone decides.
        if start == stop {
            return true;
        }
        let now = IcbTime::now().to_pcboard_time();
        if start <= stop {
            now >= start && now <= stop
        } else {
            now >= start || now <= stop
        }
    }

    /// PCBoard offers the comment only to users who may leave one, and it goes
    /// straight into the editor rather than asking a second time.
    async fn ask_comment_instead(&mut self) -> Res<()> {
        self.session.paged_sysop = true;
        let sec = self.session.user_command_level.cmd_c.clone();
        if !sec.session_can_access(&self.session) {
            return Ok(());
        }
        let comment = self
            .input_field(
                IceText::CommentInstead,
                1,
                "",
                "",
                Some(self.session.no_char.to_uppercase().to_string()),
                display_flags::YESNO
                    | display_flags::UPCASE
                    | display_flags::NEWLINE
                    | display_flags::LFBEFORE
                    | display_flags::LFAFTER
                    | display_flags::FIELDLEN,
            )
            .await?;
        if comment == self.session.yes_char.to_uppercase().to_string() {
            self.enter_comment_to_sysop().await?;
        }
        Ok(())
    }
}
