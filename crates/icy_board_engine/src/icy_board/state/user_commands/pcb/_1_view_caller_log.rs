use std::path::PathBuf;

use crate::icy_board::commands::CommandType;
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::functions::{MASK_ASCII, MASK_COMMAND, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Sysop command 1 - view, print, scan or delete the caller log.
    pub async fn view_caller_log(&mut self) -> Res<()> {
        let answer = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::ViewCallers,
                1,
                "VPSD",
                CommandType::ViewCallerLog.get_help(),
                None,
                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?
        };

        match answer.to_ascii_uppercase().as_str() {
            "V" => self.show_caller_log(None, None).await,
            "P" => {
                // PCBoard sends this one to the printer; without one the closest
                // thing is a listing that does not stop.
                self.session.disp_options.force_non_stop();
                self.show_caller_log(None, None).await
            }
            "S" => {
                let search = self.ask_log_search_text().await?;
                if search.is_empty() {
                    return Ok(());
                }
                self.show_caller_log(Some(&search), None).await
            }
            "D" => self.delete_caller_log().await,
            _ => Ok(()),
        }
    }

    /// Sysop command 13 - the caller log of one node. PCBoard had a log file per
    /// node, icy_board has one file whose lines carry the node they came from.
    pub async fn view_node_caller_log(&mut self) -> Res<()> {
        let mut node = None;
        let mut scan_all = false;
        let mut search = String::new();

        loop {
            while let Some(token) = self.session.tokens.pop_front() {
                if let Ok(number) = token.parse::<usize>() {
                    if number >= 1 && number <= self.node_state.lock().await.len() {
                        node = Some(number);
                    }
                } else if token.eq_ignore_ascii_case("S") {
                    if search.is_empty() {
                        search = self.ask_log_search_text().await?;
                    }
                } else if token.eq_ignore_ascii_case("A") {
                    scan_all = true;
                } else {
                    search = token;
                }
            }
            if node.is_some() || scan_all {
                break;
            }

            self.node_list().await?;
            let answer = self
                .input_field(
                    IceText::NodeToView,
                    9,
                    MASK_COMMAND,
                    CommandType::NodeCallerLog.get_help(),
                    None,
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if answer.is_empty() {
                return Ok(());
            }
            self.session.push_tokens(&answer);
        }

        let search = if search.is_empty() { None } else { Some(search) };
        self.show_caller_log(search.as_deref(), if scan_all { None } else { node }).await
    }

    async fn ask_log_search_text(&mut self) -> Res<String> {
        self.input_field(
            IceText::TextToScanFor,
            40,
            &MASK_ASCII,
            "hlpsrch",
            None,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::HIGHASCII,
        )
        .await
    }

    async fn caller_log_path(&self) -> PathBuf {
        let board = self.get_board().await;
        board.resolve_file(&board.config.paths.caller_log)
    }

    async fn delete_caller_log(&mut self) -> Res<()> {
        let answer = self
            .input_field(
                IceText::DeleteCallersLog,
                1,
                "",
                "",
                Some(self.session.no_char.to_uppercase().to_string()),
                display_flags::YESNO | display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        if answer != self.session.yes_char.to_uppercase().to_string() {
            return Ok(());
        }
        let path = self.caller_log_path().await;
        if path.is_file() {
            if let Err(err) = std::fs::remove_file(&path) {
                log::error!("Can't delete caller log {}: {}", path.display(), err);
                return Ok(());
            }
        }
        // PCBoard reopens the log and writes the caller back into it.
        let name = self.session.user_name.clone();
        self.write_caller_log(&format!("{name} deleted the caller log")).await;
        Ok(())
    }

    async fn show_caller_log(&mut self, search: Option<&str>, node: Option<usize>) -> Res<()> {
        let path = self.caller_log_path().await;
        let Ok(content) = std::fs::read(&path) else {
            self.display_text(IceText::NoFilesFound, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };
        let content = crate::tables::import_cp437_string(&content, false);

        let node_tag = node.map(|n| format!("[{}]", n));
        let search = search.map(|s| s.to_ascii_uppercase());

        self.new_line().await?;
        self.session.disp_options.force_count_lines();
        for line in content.lines() {
            if let Some(tag) = &node_tag {
                if !line.contains(tag.as_str()) {
                    continue;
                }
            }
            if let Some(search) = &search {
                if !line.to_ascii_uppercase().contains(search.as_str()) {
                    continue;
                }
            }
            self.print(TerminalTarget::Both, line).await?;
            self.new_line().await?;
            if self.session.disp_options.abort_printout {
                break;
            }
        }
        self.display_text(IceText::TextFileViewed, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        Ok(())
    }
}
