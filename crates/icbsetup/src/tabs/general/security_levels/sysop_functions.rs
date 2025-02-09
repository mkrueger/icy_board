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

pub struct SysopFunctions {
    menu: ICBConfigMenuUI,
}

impl SysopFunctions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 28;
            let entry: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
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
            menu: ICBConfigMenuUI::new(get_text("sysop_functions_title"), menu),
        }
    }
}

impl Page for SysopFunctions {
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
