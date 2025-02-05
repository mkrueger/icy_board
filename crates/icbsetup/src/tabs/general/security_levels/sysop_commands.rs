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

pub struct SysopCommands {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl SysopCommands {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 60;
            let entry = vec![
                cfg_entry_sec_level!("sysop_sec_level", label_width, sysop_command_level, sysop, lock),
                cfg_entry_sec_level!("sysop_sec_read_all_commentsl", label_width, sysop_command_level, read_all_comments, lock),
                cfg_entry_sec_level!("sysop_sec_read_all_mail", label_width, sysop_command_level, read_all_mail, lock),
                cfg_entry_sec_level!("sysop_sec_copy_move_messages", label_width, sysop_command_level, copy_move_messages, lock),
                cfg_entry_sec_level!(
                    "sysop_sec_enter_color_codes_in_messages",
                    label_width,
                    sysop_command_level,
                    enter_color_codes_in_messages,
                    lock
                ),
                cfg_entry_sec_level!("sysop_sec_edit_any_message", label_width, sysop_command_level, edit_any_message, lock),
                cfg_entry_sec_level!("sysop_sec_not_update_msg_read", label_width, sysop_command_level, not_update_msg_read, lock),
                cfg_entry_sec_level!("sysop_sec_use_broadcast_command", label_width, sysop_command_level, use_broadcast_command, lock),
                cfg_entry_sec_level!("sysop_sec_view_private_uploads", label_width, sysop_command_level, view_private_uploads, lock),
                cfg_entry_sec_level!(
                    "sysop_sec_enter_generic_messages",
                    label_width,
                    sysop_command_level,
                    enter_generic_messages,
                    lock
                ),
                cfg_entry_sec_level!("sysop_sec_edit_message_headers", label_width, sysop_command_level, edit_message_headers, lock),
                cfg_entry_sec_level!(
                    "sysop_sec_protect_unprotect_messages",
                    label_width,
                    sysop_command_level,
                    protect_unprotect_messages,
                    lock
                ),
                cfg_entry_sec_level!(
                    "sysop_sec_overwrite_files_on_uploads",
                    label_width,
                    sysop_command_level,
                    overwrite_files_on_uploads,
                    lock
                ),
                cfg_entry_sec_level!(
                    "sysop_sec_set_pack_out_date_on_messages",
                    label_width,
                    sysop_command_level,
                    set_pack_out_date_on_messages,
                    lock
                ),
                cfg_entry_sec_level!(
                    "sysop_sec_see_all_return_receipts",
                    label_width,
                    sysop_command_level,
                    see_all_return_receipts,
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

impl Page for SysopCommands {
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

        let val = get_text("sysop_commands_title");
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
