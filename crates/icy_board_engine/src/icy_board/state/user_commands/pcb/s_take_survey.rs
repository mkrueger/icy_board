use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::{
    Res,
    icy_board::{commands::CommandType, state::IcyBoardState},
};

use crate::{
    icy_board::{
        bulletins::MASK_BULLETINS,
        icb_config::IcbColor,
        icb_text::IceText,
        read_with_encoding_detection,
        state::{
            NodeStatus,
            functions::{MASK_ALNUM, display_flags},
        },
    },
    vm::TerminalTarget,
};
use chrono::Local;

impl IcyBoardState {
    pub async fn take_survey(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::TakeSurvey).await;
        let surveys = self.session.current_conference.surveys.clone().unwrap_or_default();
        if surveys.is_empty() {
            self.display_text(
                IceText::NoSurveysAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        let mut display_current_menu = self.session.tokens.is_empty();
        loop {
            if display_current_menu {
                let file = self.session.current_conference.survey_menu.clone();
                self.session.disp_options.no_change();
                self.display_file(&file).await?;
                display_current_menu = false;
            }
            let text = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.input_field(
                    IceText::QuestionNumberToAnswer,
                    12,
                    MASK_BULLETINS,
                    CommandType::Survey.get_help(),
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
                        break;
                    } else {
                        self.display_text(
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

    pub async fn start_survey(&mut self, survey: &crate::icy_board::surveys::Survey) -> Res<()> {
        let question = &survey.survey_file;
        let answer_file = &survey.answer_file;

        if !answer_file.exists() || !answer_file.is_file() {
            self.display_file(&question).await?;
            return Ok(());
        }

        let mut output = Vec::new();
        output.push("**************************************************************".to_string());
        if let Some(user) = &self.session.current_user {
            output.push(format!(
                "From: {}, {} Sec {} Exp {}",
                user.get_name(),
                format!(
                    "{} {}",
                    Local::now().format(&self.get_board().await.config.board.date_format),
                    Local::now().format("(%H:%M)")
                ),
                self.session.cur_security,
                user.exp_date
            ));
        } else {
            output.push(format!(
                "From: {}, {} Sec {}",
                self.session.user_name,
                format!(
                    "{} {}",
                    Local::now().format(&self.get_board().await.config.board.date_format),
                    Local::now().format("(%H:%M)")
                ),
                self.session.cur_security,
            ));
        }

        if let Some(ext) = question.extension() {
            if ext == "ppe" {
                let answer = answer_file.to_string_lossy().to_string();
                if !answer.is_empty() {
                    self.session.tokens.push_back(answer);
                }
                let t = temp_file::empty();
                self.run_ppe(&question, Some(t.path())).await?;
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
                self.reset_color(TerminalTarget::Both).await?;
                let mut start_line = 0;
                for line in &lines {
                    start_line += 1;
                    if line.starts_with("*****") {
                        break;
                    }
                    self.print(crate::vm::TerminalTarget::Both, line).await?;
                    self.new_line().await?;
                }
                let txt = if let Some(text) = self.session.tokens.pop_front() {
                    text
                } else {
                    self.input_field(
                        IceText::CompleteQuestion,
                        1,
                        "",
                        "",
                        Some(self.session.no_char.to_string()),
                        display_flags::YESNO | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::UPCASE,
                    )
                    .await?
                };
                if txt.eq_ignore_ascii_case(&self.session.yes_char.to_string()) {
                    for question in &lines[start_line..] {
                        self.set_color(TerminalTarget::Both, IcbColor::dos_yellow()).await?;
                        self.print(TerminalTarget::Both, question).await?;
                        self.new_line().await?;
                        self.reset_color(TerminalTarget::Both).await?;
                        let answer = self
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
                self.display_text(
                    IceText::ErrorReadingSurvey,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
            }
        }
        Ok(())
    }
}
