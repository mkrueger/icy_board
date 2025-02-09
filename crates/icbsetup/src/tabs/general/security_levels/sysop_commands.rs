use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_sec_level, cfg_entry_u8,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct SysopCommands {
    menu: ICBConfigMenuUI,
}

impl SysopCommands {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 52;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_u8!("sysop_sec_level", label_width, 0, 255, sysop_command_level, sysop, lock),
                cfg_entry_sec_level!("sysop_sec_read_all_comments", label_width, sysop_command_level, read_all_comments, lock),
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
            menu: ICBConfigMenuUI::new(get_text("sysop_commands_title"), menu),
        }
    }
}

impl Page for SysopCommands {
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
