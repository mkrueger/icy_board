use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::{menu_runner::PcbBoardCommand, Res};

use chrono::Local;
use icy_board_engine::{
    icy_board::{
        bulletins::MASK_BULLETINS,
        commands::Command,
        icb_config::IcbColor,
        icb_text::IceText,
        read_with_encoding_detection,
        state::{
            functions::{display_flags, MASK_ALNUM},
            UserActivity,
        },
    },
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub async fn take_survey(&mut self, action: &Command) -> Res<()> {
        self.state.set_activity(UserActivity::TakeSurvey);

        let surveys = self.state.load_surveys()?;
        if surveys.is_empty() {
            self.state
                .display_text(
                    IceText::NoSurveysAvailable,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
                )
                .await?;
            return Ok(());
        }
        let mut display_menu = self.state.session.tokens.is_empty();
        loop {
            if display_menu {
                let file = self.state.session.current_conference.survey_menu.clone();
                self.state.display_file(&file).await?;
                display_menu = false;
            }
            let text = if let Some(token) = self.state.session.tokens.pop_front() {
                token
            } else {
                self.state
                    .input_field(
                        IceText::QuestionNumberToAnswer,
                        12,
                        MASK_BULLETINS,
                        &action.help,
                        None,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                    )
                    .await?
            };
            if text.is_empty() {
                break;
            }
            if let Ok(number) = text.parse::<usize>() {
                if number > 0 {
                    if let Some(survey) = surveys.get(number - 1) {
                        self.start_survey(&survey).await?;
                        self.state.press_enter().await?;
                        self.display_menu = true;
                        break;
                    } else {
                        self.state
                            .display_text(
                                IceText::InvalidSelection,
                                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                            )
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn start_survey(&mut self, survey: &icy_board_engine::icy_board::surveys::Survey) -> Res<()> {
        let question = self.state.resolve_path(&survey.question_file);
        let answer_file = self.state.resolve_path(&survey.answer_file);

        let mut output = Vec::new();
        output.push("**************************************************************".to_string());
        if let Some(user) = &self.state.current_user {
            output.push(format!(
                "From: {}, {} Sec {} Exp {}",
                user.get_name(),
                format!(
                    "{} {}",
                    Local::now().format(&self.state.board.lock().unwrap().config.board.date_format),
                    Local::now().format("(%H:%M)")
                ),
                self.state.session.cur_security,
                user.exp_date
            ));
        } else {
            output.push(format!(
                "From: {}, {} Sec {}",
                self.state.session.user_name,
                format!(
                    "{} {}",
                    Local::now().format(&self.state.board.lock().unwrap().config.board.date_format),
                    Local::now().format("(%H:%M)")
                ),
                self.state.session.cur_security,
            ));
        }

        if let Some(ext) = question.extension() {
            if ext == "ppe" {
                let answer = answer_file.to_string_lossy().to_string();
                if !answer.is_empty() {
                    self.state.session.tokens.push_back(answer);
                }
                let t = temp_file::empty();
                self.state.run_ppe(&question, Some(t.path())).await?;
                output.push(fs::read_to_string(t.path())?);

                match OpenOptions::new().create(true).append(true).open(&answer_file) {
                    Ok(mut file) => {
                        for line in output {
                            writeln!(file, "{}", line)?;
                        }
                    }
                    Err(err) => {
                        log::error!("Error opening answer file {} : {}", answer_file.display(), err);
                        return Err(err.into());
                    }
                }
                return Ok(());
            }
        }

        match read_with_encoding_detection(&question) {
            Ok(question) => {
                let lines: Vec<&str> = question.lines().collect();
                self.state.reset_color().await?;
                let mut start_line = 0;
                for line in &lines {
                    start_line += 1;
                    if line.starts_with("*****") {
                        break;
                    }
                    self.state.print(icy_board_engine::vm::TerminalTarget::Both, line).await?;
                    self.state.new_line().await?;
                }
                let txt = if let Some(text) = self.state.session.tokens.pop_front() {
                    text
                } else {
                    self.state
                        .input_field(
                            IceText::CompleteQuestion,
                            1,
                            "",
                            "",
                            Some(self.state.session.no_char.to_string()),
                            display_flags::YESNO | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::UPCASE,
                        )
                        .await?
                };
                if txt.eq_ignore_ascii_case(&self.state.session.yes_char.to_string()) {
                    for question in &lines[start_line..] {
                        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(14)).await?;
                        self.state.print(TerminalTarget::Both, question).await?;
                        self.state.new_line().await?;
                        self.state.reset_color().await?;
                        let answer = self
                            .state
                            .input_string(
                                IcbColor::None,
                                String::new(),
                                60,
                                &MASK_ALNUM,
                                "",
                                None,
                                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::GUIDE | display_flags::LFAFTER,
                            )
                            .await?;
                        output.push(format!("Q: {}", question));
                        output.push(format!("A: {}", answer));
                    }
                    match OpenOptions::new().create(true).append(true).open(&answer_file) {
                        Ok(mut file) => {
                            for line in output {
                                writeln!(file, "{}", line)?;
                            }
                        }
                        Err(err) => {
                            log::error!("Error opening answer file {} : {}", answer_file.display(), err);
                            return Err(err.into());
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Error reading survey question: {} ({})", e, question.display());
                self.state
                    .display_text(
                        IceText::ErrorReadingSurvey,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
