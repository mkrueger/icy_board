use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{IcyBoard, ftn::FtnLogLevel};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
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

/// One of the screens `PCBoard` reached from its Fido Configuration menu.
pub struct FtnOptionPage {
    menu: ICBConfigMenuUI,
}

impl Page for FtnOptionPage {
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

fn page(title: &str, icy_board: &Arc<Mutex<IcyBoard>>, entry: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>>) -> FtnOptionPage {
    FtnOptionPage {
        menu: ICBConfigMenuUI::new(get_text(title), ConfigMenu { obj: icy_board.clone(), entry }),
    }
}

pub fn processing(icy_board: Arc<Mutex<IcyBoard>>) -> FtnOptionPage {
    let entry = {
        let lock = icy_board.lock().unwrap();
        let width = 38;
        vec![
            flag!("fido_enabled", width, enabled, lock),
            ConfigEntry::Separator,
            flag!("fido_import_after_xfer", width, import_after_xfer, lock),
            flag!("fido_process_in", width, process_in, lock),
            flag!("fido_process_out", width, process_out, lock),
            flag!("fido_dial_out", width, dial_out, lock),
            flag!("fido_process_orphan", width, process_orphan, lock),
            ConfigEntry::Separator,
            number!("fido_default_zone", width, u16::MAX as u32, default_zone, u16, lock),
            number!("fido_default_net", width, u16::MAX as u32, default_net, u16, lock),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("fido_log_level"),
                    ListValue::ComboBox(ComboBox {
                        cur_value: ComboBoxValue::new(log_level_name(lock.ftn.options.log_level), format!("{:?}", lock.ftn.options.log_level)),
                        selected_item: 0,
                        is_edit_open: false,
                        first_item: 0,
                        values: vec![
                            ComboBoxValue::new(log_level_name(FtnLogLevel::Normal), "Normal"),
                            ComboBoxValue::new(log_level_name(FtnLogLevel::Detailed), "Detailed"),
                            ComboBoxValue::new(log_level_name(FtnLogLevel::Debug), "Debug"),
                        ],
                    }),
                )
                .with_status(get_text("fido_log_level-status"))
                .with_help(get_text("fido_log_level-help"))
                .with_label_width(width)
                .with_update_combobox_value(&|board: &Arc<Mutex<IcyBoard>>, combo: &ComboBox| {
                    board.lock().unwrap().ftn.options.log_level = match combo.cur_value.value.as_str() {
                        "Detailed" => FtnLogLevel::Detailed,
                        "Debug" => FtnLogLevel::Debug,
                        _ => FtnLogLevel::Normal,
                    };
                }),
            ),
        ]
    };
    page("fido_processing_title", &icy_board, entry)
}

fn log_level_name(level: FtnLogLevel) -> String {
    match level {
        FtnLogLevel::Normal => get_text("fido_log_level_normal"),
        FtnLogLevel::Detailed => get_text("fido_log_level_detailed"),
        FtnLogLevel::Debug => get_text("fido_log_level_debug"),
    }
}

pub fn tosser(icy_board: Arc<Mutex<IcyBoard>>) -> FtnOptionPage {
    let entry = {
        let lock = icy_board.lock().unwrap();
        let width = 38;
        vec![
            flag!("fido_enable_routing", width, enable_routing, lock),
            flag!("fido_secure", width, secure, lock),
            flag!("fido_sysop_change", width, sysop_change, lock),
            ConfigEntry::Separator,
            flag!("fido_check_dupe_path", width, check_dupe_path, lock),
            flag!("fido_check_dupe_msg_id", width, check_dupe_msg_id, lock),
            number!("fido_msgs_to_track", width, 100_000, msgs_to_track, u32, lock),
            ConfigEntry::Separator,
            flag!("fido_pass_thru", width, pass_thru, lock),
            flag!("fido_make_response", width, make_response, lock),
            flag!("fido_area_fix_forwarding", width, area_fix_forwarding, lock),
            flag!("fido_auto_add_passthru", width, auto_add_passthru, lock),
            flag!("fido_re_address", width, re_address, lock),
            flag!("fido_route_echo_mail", width, route_echo_mail, lock),
            ConfigEntry::Separator,
            flag!("fido_auto_add", width, auto_add, lock),
            number!("fido_auto_add_conference", width, u16::MAX as u32, auto_add_conference, usize, lock),
        ]
    };
    page("fido_tosser_title", &icy_board, entry)
}

pub fn directories(icy_board: Arc<Mutex<IcyBoard>>) -> FtnOptionPage {
    let entry = {
        let lock = icy_board.lock().unwrap();
        let width = 28;
        vec![
            path!("fido_inbound", width, inbound, lock),
            path!("fido_outbound", width, outbound, lock),
            path!("fido_netmail", width, netmail, lock),
            path!("fido_bad_netmail", width, bad_netmail, lock),
            path!("fido_new_areas", width, new_areas, lock),
        ]
    };
    page("fido_directory_title", &icy_board, entry)
}

pub fn origin(icy_board: Arc<Mutex<IcyBoard>>) -> FtnOptionPage {
    let entry = {
        let lock = icy_board.lock().unwrap();
        vec![ConfigEntry::Item(
            ListItem::new(get_text("fido_origin"), ListValue::Text(60, TextFlags::None, lock.ftn.origin.clone()))
                .with_status(get_text("fido_origin-status"))
                .with_help(get_text("fido_origin-help"))
                .with_label_width(12)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                    board.lock().unwrap().ftn.origin = value;
                }),
        )]
    };
    page("fido_origin_title", &icy_board, entry)
}

pub fn freq(icy_board: Arc<Mutex<IcyBoard>>) -> FtnOptionPage {
    let entry = {
        let lock = icy_board.lock().unwrap();
        let width = 38;
        vec![
            ConfigEntry::Item(
                ListItem::new(get_text("fido_freq_enabled"), ListValue::Bool(lock.ftn.freq.enabled))
                    .with_status(get_text("fido_freq_enabled-status"))
                    .with_help(get_text("fido_freq_enabled-help"))
                    .with_label_width(width)
                    .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value: bool| {
                        board.lock().unwrap().ftn.freq.enabled = value;
                    }),
            ),
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    get_text("fido_freq_session_kbytes"),
                    ListValue::U32((lock.ftn.freq.limits.session_bytes / 1024) as u32, 0, u32::MAX),
                )
                .with_status(get_text("fido_freq_session_kbytes-status"))
                .with_help(get_text("fido_freq_session_kbytes-help"))
                .with_label_width(width)
                .with_update_u32_value(&|board: &Arc<Mutex<IcyBoard>>, value: u32| {
                    board.lock().unwrap().ftn.freq.limits.session_bytes = u64::from(value) * 1024;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("fido_freq_daily_kbytes"),
                    ListValue::U32((lock.ftn.freq.limits.daily_bytes / 1024) as u32, 0, u32::MAX),
                )
                .with_status(get_text("fido_freq_daily_kbytes-status"))
                .with_help(get_text("fido_freq_daily_kbytes-help"))
                .with_label_width(width)
                .with_update_u32_value(&|board: &Arc<Mutex<IcyBoard>>, value: u32| {
                    board.lock().unwrap().ftn.freq.limits.daily_bytes = u64::from(value) * 1024;
                }),
            ),
        ]
    };
    page("fido_freq_title", &icy_board, entry)
}
