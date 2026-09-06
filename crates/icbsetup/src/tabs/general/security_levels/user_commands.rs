use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_sec_level,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct UserCommands {
    menu: ICBConfigMenuUI,
}

impl UserCommands {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 31;
            let mut entry = vec![
                ConfigEntry::Table(
                    2,
                    vec![
                        cfg_entry_sec_level!("user_sec_cmd_a", label_width, user_command_level, cmd_a, lock),
                        cfg_entry_sec_level!("user_sec_cmd_b", label_width, user_command_level, cmd_b, lock),
                        cfg_entry_sec_level!("user_sec_cmd_c", label_width, user_command_level, cmd_c, lock),
                        cfg_entry_sec_level!("user_sec_cmd_d", label_width, user_command_level, cmd_d, lock),
                        cfg_entry_sec_level!("user_sec_cmd_e", label_width, user_command_level, cmd_e, lock),
                        cfg_entry_sec_level!("user_sec_cmd_f", label_width, user_command_level, cmd_f, lock),
                        cfg_entry_sec_level!("user_sec_cmd_h", label_width, user_command_level, cmd_h, lock),
                        cfg_entry_sec_level!("user_sec_cmd_i", label_width, user_command_level, cmd_i, lock),
                        cfg_entry_sec_level!("user_sec_cmd_j", label_width, user_command_level, cmd_j, lock),
                        cfg_entry_sec_level!("user_sec_cmd_k", label_width, user_command_level, cmd_k, lock),
                        cfg_entry_sec_level!("user_sec_cmd_l", label_width, user_command_level, cmd_l, lock),
                        cfg_entry_sec_level!("user_sec_cmd_m", label_width, user_command_level, cmd_m, lock),
                        cfg_entry_sec_level!("user_sec_cmd_n", label_width, user_command_level, cmd_n, lock),
                        cfg_entry_sec_level!("user_sec_cmd_o", label_width, user_command_level, cmd_o, lock),
                        cfg_entry_sec_level!("user_sec_cmd_p", label_width, user_command_level, cmd_p, lock),
                        cfg_entry_sec_level!("user_sec_cmd_q", label_width, user_command_level, cmd_q, lock),
                        cfg_entry_sec_level!("user_sec_cmd_r", label_width, user_command_level, cmd_r, lock),
                        cfg_entry_sec_level!("user_sec_cmd_s", label_width, user_command_level, cmd_s, lock),
                        cfg_entry_sec_level!("user_sec_cmd_t", label_width, user_command_level, cmd_t, lock),
                        cfg_entry_sec_level!("user_sec_cmd_u", label_width, user_command_level, cmd_u, lock),
                        cfg_entry_sec_level!("user_sec_cmd_v", label_width, user_command_level, cmd_v, lock),
                        cfg_entry_sec_level!("user_sec_cmd_w", label_width, user_command_level, cmd_w, lock),
                        cfg_entry_sec_level!("user_sec_cmd_x", label_width, user_command_level, cmd_x, lock),
                        cfg_entry_sec_level!("user_sec_cmd_y", label_width, user_command_level, cmd_y, lock),
                        cfg_entry_sec_level!("user_sec_cmd_z", label_width, user_command_level, cmd_z, lock),
                        cfg_entry_sec_level!("user_sec_cmd_chat", label_width, user_command_level, cmd_chat, lock),
                        cfg_entry_sec_level!("user_sec_cmd_open_door", label_width, user_command_level, cmd_open_door, lock),
                        cfg_entry_sec_level!("user_sec_cmd_test_file", label_width, user_command_level, cmd_test_file, lock),
                        cfg_entry_sec_level!("user_sec_cmd_show_user_list", label_width, user_command_level, cmd_show_user_list, lock),
                        cfg_entry_sec_level!("user_sec_cmd_who", label_width, user_command_level, cmd_who, lock),
                    ],
                ),
                ConfigEntry::Separator,
                cfg_entry_sec_level!("user_sec_batch_file_transfer", 42, user_command_level, batch_file_transfer, lock),
                cfg_entry_sec_level!("user_sec_edit_own_messages", 42, user_command_level, edit_own_messages, lock),
            ];
            // Two columns, including separators and four-cell security editors,
            // fit inside an 80-column page without shortening translated labels.
            if let ConfigEntry::Table(_, commands) = &mut entry[0] {
                *commands = std::mem::take(commands)
                    .into_iter()
                    .enumerate()
                    .map(|(index, command)| match command {
                        ConfigEntry::Item(item) => ConfigEntry::Item(item.with_label_width(if index % 2 == 0 { 31 } else { 28 }).with_edit_width(4)),
                        other => other,
                    })
                    .collect();
            }
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("user_commands_title"), menu),
        }
    }
}

impl Page for UserCommands {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        self.menu.render(frame, disp_area)
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_command_labels_fit_at_80_columns() {
        crate::tabs::general::layout_tests::assert_labels_fit(
            UserCommands::new(Arc::new(Mutex::new(IcyBoard::default()))),
            &[
                "user_sec_cmd_a",
                "user_sec_cmd_b",
                "user_sec_cmd_c",
                "user_sec_cmd_d",
                "user_sec_cmd_e",
                "user_sec_cmd_f",
                "user_sec_cmd_h",
                "user_sec_cmd_i",
                "user_sec_cmd_j",
                "user_sec_cmd_k",
                "user_sec_cmd_l",
                "user_sec_cmd_m",
                "user_sec_cmd_n",
                "user_sec_cmd_o",
                "user_sec_cmd_p",
                "user_sec_cmd_q",
                "user_sec_cmd_r",
                "user_sec_cmd_s",
                "user_sec_cmd_t",
                "user_sec_cmd_u",
                "user_sec_cmd_v",
                "user_sec_cmd_w",
                "user_sec_cmd_x",
                "user_sec_cmd_y",
                "user_sec_cmd_z",
                "user_sec_cmd_chat",
                "user_sec_cmd_open_door",
                "user_sec_cmd_test_file",
                "user_sec_cmd_show_user_list",
                "user_sec_cmd_who",
                "user_sec_batch_file_transfer",
                "user_sec_edit_own_messages",
            ],
        );
    }
}
