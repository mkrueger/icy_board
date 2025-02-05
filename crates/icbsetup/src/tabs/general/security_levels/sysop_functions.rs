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

pub struct SysopFunctions {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl SysopFunctions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 50;
            let entry: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                cfg_entry_sec_level!("sysop_sec_1_view_caller_log", label_width, sysop_command_level, sec_1_view_caller_log, lock),
                cfg_entry_sec_level!("sysop_sec_2_view_usr_list", label_width, sysop_command_level, sec_2_view_usr_list, lock),
                cfg_entry_sec_level!("sysop_sec_3_pack_renumber_msg", label_width, sysop_command_level, sec_3_pack_renumber_msg, lock),
                cfg_entry_sec_level!(
                    "sysop_sec_4_recover_deleted_msg",
                    label_width,
                    sysop_command_level,
                    sec_4_recover_deleted_msg,
                    lock
                ),
                cfg_entry_sec_level!("sysop_sec_5_list_message_hdr", label_width, sysop_command_level, sec_5_list_message_hdr, lock),
                cfg_entry_sec_level!("sysop_sec_6_view_any_file", label_width, sysop_command_level, sec_6_view_any_file, lock),
                cfg_entry_sec_level!("sysop_sec_7_user_maint", label_width, sysop_command_level, sec_7_user_maint, lock),
                cfg_entry_sec_level!("sysop_sec_8_pack_usr_file", label_width, sysop_command_level, sec_8_pack_usr_file, lock),
                cfg_entry_sec_level!("sysop_sec_9_exit_to_dos", label_width, sysop_command_level, sec_9_exit_to_dos, lock),
                cfg_entry_sec_level!("sysop_sec_10_shelled_dos_func", label_width, sysop_command_level, sec_10_shelled_dos_func, lock),
                cfg_entry_sec_level!("sysop_sec_11_view_other_nodes", label_width, sysop_command_level, sec_11_view_other_nodes, lock),
                cfg_entry_sec_level!("sysop_sec_12_logoff_alt_node", label_width, sysop_command_level, sec_12_logoff_alt_node, lock),
                cfg_entry_sec_level!(
                    "sysop_sec_13_view_alt_node_callers",
                    label_width,
                    sysop_command_level,
                    sec_13_view_alt_node_callers,
                    lock
                ),
                cfg_entry_sec_level!(
                    "sysop_sec_14_drop_alt_node_to_dos",
                    label_width,
                    sysop_command_level,
                    sec_14_drop_alt_node_to_dos,
                    lock
                ),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for SysopFunctions {
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

        let val = get_text("sysop_functions_title");
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
