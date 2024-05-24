use icy_board_engine::{
    icy_board::{
        icb_text::IceText,
        security_expr::SecurityExpression,
        state::{functions::display_flags, UserActivity},
        surveys::Survey,
    },
    vm::TerminalTarget,
};

use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub async fn goodbye_cmd(&mut self) -> Res<()> {
        self.state.set_activity(UserActivity::Goodbye).await;
        self.displaycmdfile("g").await?;

        if !self.state.session.flagged_files.is_empty() {
            if let Some(token) = self.state.session.tokens.pop_front() {
                if token.eq_ignore_ascii_case(&self.state.session.yes_char.to_string()) {
                    self.bye_cmd().await?;
                    return Ok(());
                }
            };

            self.state
                .display_text(IceText::FilesAreFlagged, display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE)
                .await?;
            let res = self
                .state
                .input_field(
                    IceText::ContinueLogoff,
                    1,
                    "",
                    "",
                    Some(self.state.session.no_char.to_string()),
                    display_flags::YESNO | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::NEWLINE | display_flags::FIELDLEN,
                )
                .await?;

            if !res.eq_ignore_ascii_case(&self.state.session.yes_char.to_string()) {
                return Ok(());
            }
        }

        self.bye_cmd().await?;
        Ok(())
    }

    pub async fn bye_cmd(&mut self) -> Res<()> {
        let survey = {
            let board = self.state.get_board().await;
            Survey {
                survey_file: board.resolve_file(&board.config.paths.logon_survey),
                answer_file: board.resolve_file(&board.config.paths.logon_answer),
                required_security: SecurityExpression::default(),
            }
        };

        if !self.state.session.is_sysop && survey.survey_file.exists() {
            // skip the survey question.
            self.state.session.tokens.push_front(self.state.session.yes_char.to_string());
            self.start_survey(&survey).await?;
        }
        self.state
            .display_text(IceText::ThanksForCalling, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.hangup().await?;
        Ok(())
    }
}
