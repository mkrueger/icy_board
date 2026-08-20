use std::path::{Component, Path};

use chrono::{DateTime, Local};

use crate::icy_board::commands::CommandType;
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::functions::{MASK_ASCII, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Sysop command 16 - list a directory of the board. `PCBoard` shelled out to
    /// DOS for this; here it is name, size and date and nothing else.
    pub async fn dir_command(&mut self) -> Res<()> {
        let answer = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::EnterDirCommand,
                30,
                &MASK_ASCII,
                CommandType::DirCommand.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?
        };
        if answer.is_empty() {
            return Ok(());
        }

        // The path is typed by a caller and can be stuffed by a PPE, so it stays
        // inside the board directory.
        if !is_inside_board(&answer) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                .await?;
            return Ok(());
        }

        let dir = self.get_board().await.resolve_file(&answer);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            self.display_text(IceText::NoFilesFound, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };

        let mut listing: Vec<(String, u64, Option<DateTime<Local>>, bool)> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(data) = entry.metadata() else {
                continue;
            };
            let modified = data.modified().ok().map(DateTime::<Local>::from);
            listing.push((name, data.len(), modified, data.is_dir()));
        }
        if listing.is_empty() {
            self.display_text(IceText::NoFilesFound, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        listing.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));

        self.new_line().await?;
        self.session.disp_options.force_count_lines();
        let mut total = 0;
        for (name, size, modified, is_dir) in &listing {
            let size_str = if *is_dir { "<DIR>".to_string() } else { size.to_string() };
            let date_str = match modified {
                Some(date) => format!("{} {}", self.format_date(date.to_utc()), self.format_time(date.to_utc())),
                None => String::new(),
            };
            self.println(TerminalTarget::Both, &format!("{name:<40} {size_str:>12} {date_str}")).await?;
            if !is_dir {
                total += size;
            }
            if self.session.disp_options.abort_printout {
                return Ok(());
            }
        }
        self.new_line().await?;
        self.println(TerminalTarget::Both, &format!("{} file(s), {} bytes", listing.len(), total))
            .await?;
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
