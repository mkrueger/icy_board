use chrono::Utc;

use crate::{
    icy_board::{
        icb_text::IceText,
        security_expr::SecurityExpression,
        state::{Logoff, LogoffKind, NodeStatus, functions::display_flags},
        surveys::Survey,
    },
    vm::TerminalTarget,
};

use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn goodbye_cmd(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::LogoffPending).await;
        self.displaycmdfile("g").await?;
        let is_flagged = !self.session.flagged_files.is_empty();
        if self.board.lock().await.config.system_control.guard_logoff || is_flagged {
            if let Some(token) = self.session.tokens.pop_front()
                && token.eq_ignore_ascii_case(&self.session.yes_char.to_string())
            {
                self.logoff_user(Logoff::NORMAL).await?;
                return Ok(());
            }
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

        self.logoff_user(Logoff::NORMAL).await?;
        Ok(())
    }

    pub async fn bye_cmd(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::LogoffPending).await;
        self.displaycmdfile("bye").await?;
        self.logoff_user(Logoff::NORMAL).await?;
        Ok(())
    }

    pub async fn logoff_user(&mut self, logoff: Logoff) -> Res<()> {
        if self.session.logoff_started {
            return Ok(());
        }
        self.session.logoff_started = true;
        self.session.keyboard_timer_check = false;
        let result = self.logoff_survey(logoff).await;
        self.session.logoff_pending = Some(logoff);
        self.session.request_logoff = true;
        // hangup shuts down the socket. Keep it open until all enclosing
        // command/door minutes are posted and the final summary is displayed.
        let completed = if self.accounting_invocation_active() {
            self.accounting_finish().await
        } else {
            self.accounting_complete_logoff().await
        };
        result.and(completed)
    }

    async fn logoff_survey(&mut self, logoff: Logoff) -> Res<()> {
        if logoff.asks_the_caller() {
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
        Ok(())
    }

    /// Called directly for unwrapped logoff, otherwise by the outer invocation.
    /// Take the request before display so recursive @HANGUP@ cannot replay it.
    #[async_recursion::async_recursion(?Send)]
    pub(crate) async fn accounting_complete_logoff(&mut self) -> Res<()> {
        let Some(logoff) = self.session.logoff_pending.take() else {
            return Ok(());
        };
        let finalized = self.accounting_finish().await;
        // Never advertise a final balance if settlement/persistence failed.
        let displayed = if finalized.is_ok() && !self.session.accounting.invocation_settlement_failed {
            self.accounting_display_logoff(logoff).await
        } else {
            Ok(())
        };
        let closed = self.hangup().await;
        finalized.and(displayed).and(closed)
    }

    async fn accounting_display_logoff(&mut self, logoff: Logoff) -> Res<()> {
        // The line is already gone, so the closing lines can only reach the caller log.
        let offline = self.session.is_logoff_forced();
        // accounting_active is false now: these are settled, not previews.
        if !offline && self.session.accounting.begun && self.session.accounting.mode != crate::icy_board::accounting::AccountingMode::Disabled {
            if logoff.asks_the_caller() {
                let path = self.session.accounting.options.logoff_file.clone();
                if !path.as_os_str().is_empty() {
                    self.display_file(&path).await?;
                }
            }
            self.display_text(IceText::CreditsUsed, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT)
                .await?;
            if self.session.accounting.mode == crate::icy_board::accounting::AccountingMode::Enforced {
                self.display_text(IceText::CreditsLeft, display_flags::NEWLINE | display_flags::LOGIT).await?;
            }
        }
        self.session.op_text = (Utc::now() - self.session.login_date).num_minutes().to_string();
        if offline {
            let text = self.get_display_text(IceText::MinutesUsed)?.replace("@OPTEXT@", &self.session.op_text);
            log::info!("{text}");
            return Ok(());
        }
        self.display_text(IceText::MinutesUsed, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT)
            .await?;
        if logoff.kind == LogoffKind::Normal {
            self.display_text(IceText::ThanksForCalling, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.reset_color(TerminalTarget::Both).await?;
        }

        Ok(())
    }
}
