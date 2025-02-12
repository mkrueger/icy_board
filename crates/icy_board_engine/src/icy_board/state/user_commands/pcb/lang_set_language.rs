use crate::icy_board::state::functions::MASK_NUM;
use crate::icy_board::state::IcyBoardState;
use crate::Res;
use crate::{
    icy_board::{icb_config::IcbColor, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn set_language_cmd(&mut self) -> Res<()> {
        if self.displaycmdfile("lang").await? {
            return Ok(());
        }
        let cur_lang = if let Some(user) = &mut self.session.current_user {
            user.language.clone()
        } else {
            String::new()
        };

        let lang = self.ask_languages(cur_lang).await?;
        if !lang.is_empty() {
            if let Some(user) = &mut self.session.current_user {
                user.language = lang;
            }
            self.save_current_user().await?;
        }
        Ok(())
    }

    pub async fn ask_languages(&mut self, cur_language: String) -> Res<String> {
        let mut languages = Vec::new();
        self.new_line().await?;
        let l = self.get_board().await.languages.clone();
        let mut cur_lang_str = String::new();
        for (i, lang) in l.iter().enumerate() {
            if lang.extension == cur_language {
                cur_lang_str = format!("{}", i + 1);
                languages.push(format!("=> ({}) {}", i + 1, lang.description));
            } else {
                languages.push(format!("   ({}) {}", i + 1, lang.description));
            }
        }
        self.display_text(IceText::LanguageAvailable, display_flags::NEWLINE | display_flags::LFAFTER)
            .await?;

        self.set_color(TerminalTarget::Both, IcbColor::dos_cyan()).await?;
        for line in languages {
            self.print(TerminalTarget::Both, &line).await?;
            self.new_line().await?;
        }
        loop {
            let language = self
                .input_field(
                    IceText::LanguageEnterNumber,
                    3,
                    &MASK_NUM,
                    "",
                    Some(cur_lang_str.clone()),
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;
            if language.is_empty() {
                return Ok(language);
            }
            if let Ok(number) = language.parse::<usize>() {
                if number > 0 && number <= l.len() {
                    if number == 1 {
                        self.display_text(IceText::LanguageActive, display_flags::NEWLINE).await?;
                    }
                    return Ok(l[number - 1].extension.clone());
                }
            }
            self.display_text(IceText::LanguageNotAvailable, display_flags::NEWLINE).await?;
        }
    }
}
