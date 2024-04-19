use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::Res;
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

use super::PcbBoardCommand;

impl PcbBoardCommand {
    pub fn take_survey(&mut self, action: &Command) -> Res<()> {
        self.state.node_state.lock().unwrap().user_activity = UserActivity::TakeSurvey;

        let surveys = self.state.load_surveys()?;
        if surveys.is_empty() {
            self.state.display_text(
                IceText::NoSurveysAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
            )?;
            return Ok(());
        }
        let mut display_menu = self.state.session.tokens.is_empty();
        loop {
            if display_menu {
                let file = self.state.session.current_conference.survey_menu.clone();
                self.state.display_file(&file)?;
                display_menu = false;
            }
            let text = if let Some(token) = self.state.session.tokens.pop_front() {
                token
            } else {
                self.state.input_field(
                    IceText::QuestionNumberToAnswer,
                    12,
                    MASK_BULLETINS,
                    &action.help,
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                )?
            };
            if text.is_empty() {
                break;
            }
            if let Ok(number) = text.parse::<usize>() {
                if number > 0 {
                    if let Some(survey) = surveys.get(number - 1) {
                        self.start_survey(&survey)?;
                        break;
                    } else {
                        self.state.display_text(
                            IceText::InvalidSelection,
                            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                        )?;
                    }
                }
            }
        }
        self.display_menu = true;
        Ok(())
    }

    fn start_survey(&mut self, survey: &&icy_board_engine::icy_board::surveys::Survey) -> Res<()> {
        let question = self.state.resolve_path(&survey.question_file);
        let answer_file = self.state.resolve_path(&survey.answer_file);

        let mut output = Vec::new();
        output.push("**************************************************************".to_string());
        if let Some(user) = &self.state.current_user {
            output.push(format!(
                "From: {}, {} Sec {} Exp {}",
                user.get_name(),
                Local::now().format("%Y/%m/%d (%H:%M)"),
                self.state.session.cur_security,
                user.exp_date
            ));
        }

        if let Some(ext) = question.extension() {
            if ext == "ppe" {
                let answer = answer_file.to_string_lossy().to_string();
                if !answer.is_empty() {
                    self.state.session.tokens.push_back(answer);
                }
                let t = temp_file::empty();
                self.state.run_ppe(&question, Some(t.path()))?;
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
                self.state.reset_color()?;
                for line in &lines[0..5] {
                    self.state.print(icy_board_engine::vm::TerminalTarget::Both, line)?;
                    self.state.new_line()?;
                }
                let txt = self.state.input_field(
                    IceText::CompleteQuestion,
                    1,
                    "",
                    "",
                    Some(self.state.session.no_char.to_string()),
                    display_flags::YESNO | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::UPCASE,
                )?;
                if txt.starts_with(self.state.session.yes_char) {
                    for question in &lines[5..] {
                        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(14))?;
                        self.state.print(TerminalTarget::Both, question)?;
                        self.state.new_line()?;
                        self.state.reset_color()?;
                        let answer = self.state.input_string(
                            IcbColor::None,
                            String::new(),
                            60,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::GUIDE | display_flags::LFAFTER,
                        )?;
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
                self.state.display_text(
                    IceText::ErrorReadingSurvey,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )?;
            }
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
