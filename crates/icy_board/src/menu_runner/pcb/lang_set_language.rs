use crate::{
    menu_runner::{PcbBoardCommand, MASK_NUMBER},
    Res,
};
use icy_board_engine::{
    icy_board::{commands::Command, icb_config::IcbColor, icb_text::IceText, state::functions::display_flags, IcyBoardError},
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub fn set_language(&mut self, _action: &Command) -> Res<()> {
        if self.displaycmdfile("lang")? {
            return Ok(());
        }
        let cur_lang = if let Some(user) = &mut self.state.current_user {
            user.language.clone()
        } else {
            String::new()
        };

        let lang = self.ask_languages(cur_lang)?;
        if let Some(user) = &mut self.state.current_user {
            user.language = lang;
        }
        self.state.save_current_user()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    pub fn ask_languages(&mut self, cur_language: String) -> Res<String> {
        let mut languages = Vec::new();
        self.state.new_line()?;
        let l = if let Ok(board) = self.state.board.lock() {
            board.languages.clone()
        } else {
            return Err(IcyBoardError::ErrorLockingBoard.into());
        };
        let mut cur_lang_str = String::new();
        for (i, lang) in l.languages.iter().enumerate() {
            if lang.extension == cur_language {
                cur_lang_str = format!("{}", i + 1);
                languages.push(format!("=> ({}) {}", i + 1, lang.description));
            } else {
                languages.push(format!("   ({}) {}", i + 1, lang.description));
            }
        }
        self.state
            .display_text(IceText::LanguageAvailable, display_flags::NEWLINE | display_flags::LFAFTER)?;

        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
        for line in languages {
            self.state.print(TerminalTarget::Both, &line)?;
            self.state.new_line()?;
        }
        loop {
            let language = self.state.input_field(
                IceText::LanguageEnterNumber,
                3,
                &MASK_NUMBER,
                "",
                Some(cur_lang_str.clone()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
            )?;

            if let Ok(number) = language.parse::<usize>() {
                if number > 0 && number <= l.languages.len() {
                    if number == 1 {
                        self.state.display_text(IceText::LanguageActive, display_flags::NEWLINE)?;
                    }
                    return Ok(l.languages[number - 1].extension.clone());
                }
            }
            self.state.display_text(IceText::LanguageNotAvailable, display_flags::NEWLINE)?;
        }
    }
}
