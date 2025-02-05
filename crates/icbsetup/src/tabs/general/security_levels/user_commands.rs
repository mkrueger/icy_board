use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::cfg_entry_sec_level;

pub struct UserCommands {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl UserCommands {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 22;
            let entry = vec![
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
                cfg_entry_sec_level!("user_sec_batch_file_transfer", label_width, user_command_level, batch_file_transfer, lock),
                cfg_entry_sec_level!("user_sec_edit_own_messages", label_width, user_command_level, edit_own_messages, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for UserCommands {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = get_text("user_commands_title");
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().menu_title).render(
            Rect {
                x: area.x + 1 + area.width.saturating_sub(width) / 2,
                y: area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );

        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().content_box,
        );
        let area = Rect {
            x: area.x + 1,
            y: area.y + 3,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(4),
        };
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        let res = self.menu.handle_key_press(key, &mut self.state);

        PageMessage::ResultState(res)
    }
}
