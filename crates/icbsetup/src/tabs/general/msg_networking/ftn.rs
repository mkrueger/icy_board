use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

/// The fidonet settings live in `ftn.toml` rather than in `icboard.toml`, and
/// the shared entry macros all reach through `config`.
macro_rules! flag {
    ($i:expr, $width:expr, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(get_text($i), ListValue::Bool($lock.ftn.options.$conf))
                .with_status(&get_text(&format!("{}-status", $i)))
                .with_help(&get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value: bool| {
                    board.lock().unwrap().ftn.options.$conf = value;
                }),
        )
    };
}

macro_rules! number {
    ($i:expr, $width:expr, $max:expr, $conf:ident, $cast:ty, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(get_text($i), ListValue::U32($lock.ftn.options.$conf as u32, 0, $max))
                .with_status(&get_text(&format!("{}-status", $i)))
                .with_help(&get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::U32(val, _, _) = value else {
                        return;
                    };
                    board.lock().unwrap().ftn.options.$conf = *val as $cast;
                })),
        )
    };
}

macro_rules! path {
    ($i:expr, $width:expr, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(get_text($i), ListValue::Path($lock.ftn.$conf.clone()))
                .with_status(&get_text(&format!("{}-status", $i)))
                .with_help(&get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::Path(val) = value else {
                        return;
                    };
                    board.lock().unwrap().ftn.$conf = val.clone();
                })),
        )
    };
}

pub struct FtnSettings {
    menu: ICBConfigMenuUI,
}

impl FtnSettings {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let width = 22;
            let entries: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("ftn_run_label")),
                flag!("ftn_process_in", width, process_in, lock),
                flag!("ftn_process_out", width, process_out, lock),
                flag!("ftn_process_orphan", width, process_orphan, lock),
                flag!("ftn_dial_out", width, dial_out, lock),
                flag!("ftn_import_after_xfer", width, import_after_xfer, lock),
                flag!("ftn_verbose_log", width, verbose_log, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("ftn_dupes_label")),
                flag!("ftn_check_dupe_msg_id", width, check_dupe_msg_id, lock),
                flag!("ftn_check_dupe_path", width, check_dupe_path, lock),
                number!("ftn_msgs_to_track", width, 100_000, msgs_to_track, u32, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("ftn_areas_label")),
                flag!("ftn_auto_add", width, auto_add, lock),
                number!("ftn_auto_add_conference", width, u16::MAX as u32, auto_add_conference, usize, lock),
                flag!("ftn_pass_thru", width, pass_thru, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("ftn_mail_label")),
                flag!("ftn_secure", width, secure, lock),
                flag!("ftn_sysop_change", width, sysop_change, lock),
                number!("ftn_default_zone", width, u16::MAX as u32, default_zone, u16, lock),
                number!("ftn_default_net", width, u16::MAX as u32, default_net, u16, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("ftn_paths_label")),
                path!("ftn_inbound", width, inbound, lock),
                path!("ftn_outbound", width, outbound, lock),
                path!("ftn_netmail", width, netmail, lock),
                path!("ftn_bad_netmail", width, bad_netmail, lock),
                path!("ftn_new_areas", width, new_areas, lock),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: entries,
            }
        };
        Self {
            menu: ICBConfigMenuUI::new(get_text("ftn_settings_title"), menu),
        }
    }
}

impl Page for FtnSettings {
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
