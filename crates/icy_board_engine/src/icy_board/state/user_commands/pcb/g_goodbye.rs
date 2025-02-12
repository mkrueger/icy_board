use crate::{
    icy_board::{
        icb_text::IceText,
        security_expr::SecurityExpression,
        state::{functions::display_flags, NodeStatus},
        surveys::Survey,
    },
    vm::TerminalTarget,
};

use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn goodbye_cmd(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::LogoffPending).await;
        self.displaycmdfile("g").await?;
        let is_flagged = !self.session.flagged_files.is_empty();
        if self.board.lock().await.config.system_control.guard_logoff || is_flagged {
            if let Some(token) = self.session.tokens.pop_front() {
                if token.eq_ignore_ascii_case(&self.session.yes_char.to_string()) {
                    self.bye_cmd(false).await?;
                    return Ok(());
                }
            };
            if is_flagged {
                self.display_text(IceText::FilesAreFlagged, display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE)
                    .await?;
            }
            let res = self
                .input_field(
                    IceText::ContinueLogoff,
                    1,
                    "",
                    "",
                    Some(self.session.no_char.to_string()),
                    display_flags::YESNO | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::NEWLINE | display_flags::FIELDLEN,
                )
                .await?;

            if !res.eq_ignore_ascii_case(&self.session.yes_char.to_string()) {
                return Ok(());
            }
        }

        self.bye_cmd(false).await?;
        Ok(())
    }

    pub async fn bye_cmd(&mut self, auto_logoff: bool) -> Res<()> {
        if !auto_logoff {
            let survey = {
                let board = self.get_board().await;
                Survey {
                    survey_file: board.resolve_file(&board.config.paths.logoff_survey),
                    answer_file: board.resolve_file(&board.config.paths.logoff_answer),
                    required_security: SecurityExpression::default(),
                }
            };

            if !self.session.is_sysop && survey.survey_file.exists() {
                // skip the survey question.
                self.session.tokens.push_front(self.session.yes_char.to_string());
                self.start_survey(&survey).await?;
            }
        }
        self.display_text(IceText::ThanksForCalling, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.reset_color(TerminalTarget::Both).await?;

        self.hangup().await?;
        Ok(())
    }
}
