use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::{PplHttpDestinationPolicy, PplHttpOptions, normalize_ppl_http_origins},
};
use icy_board_tui::{
    cfg_entry_bool,
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct PplHttp {
    menu: ICBConfigMenuUI,
    validation_error: Arc<Mutex<Option<String>>>,
}

fn policy_name(policy: &PplHttpDestinationPolicy) -> String {
    match policy {
        PplHttpDestinationPolicy::Disabled => get_text("ppl_http_policy_disabled"),
        PplHttpDestinationPolicy::Allowlist => get_text("ppl_http_policy_allowlist"),
        PplHttpDestinationPolicy::Public => get_text("ppl_http_policy_public"),
    }
}

fn numeric_entry(
    key: &'static str,
    label_width: u16,
    value: usize,
    minimum: u32,
    maximum: u32,
    update: fn(&mut PplHttpOptions, usize),
) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::U32(value.min(u32::MAX as usize) as u32, minimum, maximum))
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_label_width(label_width)
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                let ListValue::U32(value, _, _) = value else {
                    return;
                };
                update(&mut board.lock().unwrap().config.ppl_http, *value as usize);
            })),
    )
}

fn seconds_entry(key: &'static str, label_width: u16, value: u64, maximum: u32, update: fn(&mut PplHttpOptions, u64)) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::U32(value.min(u64::from(u32::MAX)) as u32, 1, maximum))
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_label_width(label_width)
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                let ListValue::U32(value, _, _) = value else {
                    return;
                };
                update(&mut board.lock().unwrap().config.ppl_http, u64::from(*value));
            })),
    )
}

impl PplHttp {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let validation_error = Arc::new(Mutex::new(None));
        let menu = {
            let lock = icy_board.lock().unwrap();
            let options = &lock.config.ppl_http;
            let label_width = 31;
            let entries = vec![
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("ppl_http_policy"),
                        ListValue::ComboBox(ComboBox {
                            cur_value: ComboBoxValue::new(policy_name(&options.destination_policy), format!("{:?}", options.destination_policy)),
                            selected_item: 0,
                            is_edit_open: false,
                            first_item: 0,
                            values: vec![
                                ComboBoxValue::new(policy_name(&PplHttpDestinationPolicy::Disabled), "Disabled"),
                                ComboBoxValue::new(policy_name(&PplHttpDestinationPolicy::Allowlist), "Allowlist"),
                                ComboBoxValue::new(policy_name(&PplHttpDestinationPolicy::Public), "Public"),
                            ],
                        }),
                    )
                    .with_status(get_text("ppl_http_policy-status"))
                    .with_help(get_text("ppl_http_policy-help"))
                    .with_label_width(label_width)
                    .with_update_combobox_value(&|board: &Arc<Mutex<IcyBoard>>, combo: &ComboBox| {
                        board.lock().unwrap().config.ppl_http.destination_policy = match combo.cur_value.value.as_str() {
                            "Allowlist" => PplHttpDestinationPolicy::Allowlist,
                            "Public" => PplHttpDestinationPolicy::Public,
                            _ => PplHttpDestinationPolicy::Disabled,
                        };
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("ppl_http_allowed_origins"),
                        ListValue::Text(255, TextFlags::None, options.allowed_origins.join(", ")),
                    )
                    .with_status(get_text("ppl_http_allowed_origins-status"))
                    .with_help(get_text("ppl_http_allowed_origins-help"))
                    .with_label_width(label_width)
                    .with_edit_width(70)
                    .with_update_value({
                        let validation_error = validation_error.clone();
                        Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                            let ListValue::Text(_, _, value) = value else {
                                return;
                            };
                            match normalize_ppl_http_origins(value) {
                                Ok(origins) => {
                                    board.lock().unwrap().config.ppl_http.allowed_origins = origins;
                                    *validation_error.lock().unwrap() = None;
                                }
                                Err(error) => *validation_error.lock().unwrap() = Some(error),
                            }
                        })
                    }),
                ),
                cfg_entry_bool!("ppl_http_allow_http", label_width, ppl_http, allow_http, lock),
                ConfigEntry::Separator,
                numeric_entry(
                    "ppl_http_max_response_bytes",
                    label_width,
                    options.max_response_bytes,
                    1,
                    u32::MAX,
                    |options, value| {
                        options.max_response_bytes = value;
                    },
                ),
                numeric_entry(
                    "ppl_http_max_request_bytes",
                    label_width,
                    options.max_request_bytes,
                    1,
                    u32::MAX,
                    |options, value| {
                        options.max_request_bytes = value;
                    },
                ),
                numeric_entry("ppl_http_max_headers", label_width, options.max_headers, 1, 1024, |options, value| {
                    options.max_headers = value;
                }),
                numeric_entry(
                    "ppl_http_max_header_bytes",
                    label_width,
                    options.max_header_bytes,
                    1024,
                    1024 * 1024,
                    |options, value| {
                        options.max_header_bytes = value;
                    },
                ),
                ConfigEntry::Separator,
                seconds_entry(
                    "ppl_http_connect_timeout",
                    label_width,
                    options.connect_timeout_seconds,
                    300,
                    |options, value| {
                        options.connect_timeout_seconds = value;
                    },
                ),
                seconds_entry(
                    "ppl_http_request_timeout",
                    label_width,
                    options.request_timeout_seconds,
                    3600,
                    |options, value| {
                        options.request_timeout_seconds = value;
                    },
                ),
                numeric_entry("ppl_http_max_redirects", label_width, options.max_redirects, 0, 20, |options, value| {
                    options.max_redirects = value;
                }),
                numeric_entry(
                    "ppl_http_max_concurrent",
                    label_width,
                    options.max_concurrent_requests,
                    1,
                    1024,
                    |options, value| {
                        options.max_concurrent_requests = value;
                    },
                ),
                numeric_entry(
                    "ppl_http_max_concurrent_node",
                    label_width,
                    options.max_concurrent_per_node,
                    1,
                    64,
                    |options, value| {
                        options.max_concurrent_per_node = value;
                    },
                ),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: entries,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("ppl_http_title"), menu),
            validation_error,
        }
    }
}

impl Page for PplHttp {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        self.menu.render(frame, area)
    }

    fn request_status(&self) -> ResultState {
        match self.validation_error.lock().unwrap().clone() {
            Some(error) => ResultState::status_line(error),
            None => self.menu.request_status(),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}
