use std::path::{Component, Path};

use crate::icy_board::commands::CommandType;
use crate::icy_board::{
    icb_text::IceText,
    state::functions::{MASK_ASCII, display_flags},
};
use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    /// Sysop command 6 - display any text file of the board.
    pub async fn view_text_file(&mut self) -> Res<()> {
        let file_name = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            let answer = self
                .input_field(
                    IceText::TextViewFileName,
                    30,
                    &MASK_ASCII,
                    CommandType::ViewTextFile.get_help(),
                    None,
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            self.new_line().await?;
            answer
        };
        if file_name.is_empty() {
            return Ok(());
        }

        // The name is typed by a caller and may be stuffed by a PPE, so it stays
        // inside the board directory instead of reaching anywhere on the host.
        if !is_inside_board(&file_name) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                .await?;
            return Ok(());
        }

        self.session.disp_options.no_change();
        if self.display_file(&file_name).await? {
            self.display_text(IceText::TextFileViewed, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        }
        Ok(())
    }
}

fn is_inside_board(file_name: &str) -> bool {
    let path = Path::new(file_name);
    if path.is_absolute() {
        return false;
    }
    path.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}
