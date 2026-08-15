use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    user_base::ConferenceFlags,
    user_maintenance::{self, ExpirationChange, MaintenanceReport, SecurityField, UserSelection},
};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListItem, ListValue, ResultState},
    get_text, get_text_args,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};
use std::collections::HashMap;

use super::{counter_init_from_option, counter_scope};

/// The bulk operations offered below "Users File Maintenance", named the way
/// the utility this replaces named them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceOp {
    Pack,
    AdjustSecurity,
    AdjustSecurityExpired,
    CopyExpiredSecurity,
    InitializeCounters,
    AdjustExpiration,
    ConferenceInsert,
    ConferenceRemove,
    ConferenceMove,
    StandardizePhones,
}

impl MaintenanceOp {
    fn title(&self) -> String {
        match self {
            MaintenanceOp::Pack => get_text("icbsm_pack_title"),
            MaintenanceOp::AdjustSecurity => get_text("icbsm_sec_by_ranges_title"),
            MaintenanceOp::AdjustSecurityExpired => get_text("icbsm_sec_by_ranges_expired_title"),
            MaintenanceOp::CopyExpiredSecurity => get_text("icbsm_sec_copy_expired"),
            MaintenanceOp::InitializeCounters => get_text("icbsm_counters_title"),
            MaintenanceOp::AdjustExpiration => get_text("icbsm_expiration_title"),
            MaintenanceOp::ConferenceInsert => get_text("icbsm_conf_insert_title"),
            MaintenanceOp::ConferenceRemove => get_text("icbsm_conf_remove_title"),
            MaintenanceOp::ConferenceMove => get_text("icbsm_conf_move_title"),
            MaintenanceOp::StandardizePhones => get_text("icbsm_phones_title"),
        }
    }

    /// Packing throws records away, everything else can be undone by editing again.
    fn removes_users(&self) -> bool {
        *self == MaintenanceOp::Pack
    }
}

#[derive(Clone)]
struct Params {
    min_security: u32,
    max_security: u32,
    use_expired_level: bool,

    remove_deleted_or_locked: bool,
    inactive_days: u32,
    last_on_since: DateTime<Utc>,
    expired_before: DateTime<Utc>,
    keep_security: u32,
    keep_locked_out: bool,

    new_level: u32,
    counter_option: u32,
    counter_files: bool,
    counter_bytes: bool,

    expiration_date: DateTime<Utc>,
    add_days: u32,

    conf_first: u32,
    conf_last: u32,
    conf_target: u32,
    flag_registered: bool,
    flag_expired: bool,
    flag_selected: bool,
    flag_sysop: bool,
    flag_net_status: bool,
    reset_lastread: bool,
    move_last_conference: bool,
}

/// The date the original used for "no date given", still the way to switch the
/// two date criteria off.
fn no_date() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(1980, 1, 1, 0, 0, 0).unwrap()
}

fn date_is_set(date: DateTime<Utc>) -> bool {
    date > no_date()
}

/// 9999 days means "do not look at the last call", as it did in the original.
const DAYS_OFF: u32 = 9999;

impl Default for Params {
    fn default() -> Self {
        Self {
            min_security: 0,
            max_security: 0,
            use_expired_level: false,
            remove_deleted_or_locked: true,
            inactive_days: DAYS_OFF,
            last_on_since: no_date(),
            expired_before: no_date(),
            keep_security: 100,
            keep_locked_out: true,
            new_level: 0,
            counter_option: 1,
            counter_files: false,
            counter_bytes: false,
            expiration_date: no_date(),
            add_days: 0,
            conf_first: 0,
            conf_last: 0,
            conf_target: 0,
            flag_registered: true,
            flag_expired: true,
            flag_selected: true,
            flag_sysop: false,
            flag_net_status: false,
            reset_lastread: false,
            move_last_conference: false,
        }
    }
}

enum Stage {
    Confirm,
    Criteria,
    Preview { matched: usize, names: Vec<String> },
    Done { report: MaintenanceReport },
}

/// Criteria, preview, then run: the same three steps for every bulk operation.
pub struct MaintenancePage {
    op: MaintenanceOp,
    icy_board: Arc<Mutex<IcyBoard>>,
    menu: ConfigMenu<Arc<Mutex<Params>>>,
    state: ConfigMenuState,
    stage: Stage,
    error: Option<String>,
}

type Obj = Arc<Mutex<Params>>;

const LABEL_WIDTH: u16 = 30;
/// The criteria are whole sentences, like the screens they come from, and each
/// screen lines its answers up in one column.
const PACK_LABEL_WIDTH: u16 = 50;
const SECURITY_LABEL_WIDTH: u16 = 55;
const EXPIRATION_LABEL_WIDTH: u16 = 61;
const CONFERENCE_LABEL_WIDTH: u16 = 60;
const COUNTER_LABEL_WIDTH: u16 = 44;

fn screen_label_width(op: MaintenanceOp) -> u16 {
    match op {
        MaintenanceOp::Pack => PACK_LABEL_WIDTH,
        MaintenanceOp::AdjustSecurity | MaintenanceOp::AdjustSecurityExpired | MaintenanceOp::CopyExpiredSecurity => SECURITY_LABEL_WIDTH,
        MaintenanceOp::AdjustExpiration => EXPIRATION_LABEL_WIDTH,
        MaintenanceOp::ConferenceInsert | MaintenanceOp::ConferenceRemove | MaintenanceOp::ConferenceMove => CONFERENCE_LABEL_WIDTH,
        _ => LABEL_WIDTH,
    }
}

fn sized(entry: ConfigEntry<Obj>, width: u16) -> ConfigEntry<Obj> {
    match entry {
        ConfigEntry::Item(item) => ConfigEntry::Item(item.with_label_width(width)),
        other => other,
    }
}

/// The second half of a sentence that started on the line above, pulled over to
/// the colon the way the original set it.
fn continued(entry: ConfigEntry<Obj>, width: u16) -> ConfigEntry<Obj> {
    match entry {
        ConfigEntry::Item(item) => ConfigEntry::Item(item.with_label_width(width).with_label_alignment(Alignment::Right)),
        other => other,
    }
}

fn u32_item(label: &str, value: u32, min: u32, max: u32, update: &'static dyn Fn(&Obj, u32)) -> ConfigEntry<Obj> {
    ConfigEntry::Item(
        ListItem::new(get_text(label), ListValue::U32(value, min, max))
            .with_status(get_text(label))
            .with_label_width(LABEL_WIDTH)
            .with_update_u32_value(update),
    )
}

fn bool_item(label: &str, value: bool, update: &'static dyn Fn(&Obj, bool)) -> ConfigEntry<Obj> {
    ConfigEntry::Item(
        ListItem::new(get_text(label), ListValue::Bool(value))
            .with_status(get_text(label))
            .with_label_width(LABEL_WIDTH)
            .with_update_bool_value(update),
    )
}

fn wide(entry: ConfigEntry<Obj>) -> ConfigEntry<Obj> {
    sized(entry, PACK_LABEL_WIDTH)
}

/// Replaces the status line of an item, for the fields that carry an off value.
fn with_hint(entry: ConfigEntry<Obj>, hint: &str) -> ConfigEntry<Obj> {
    match entry {
        ConfigEntry::Item(item) => ConfigEntry::Item(item.with_status(get_text(hint))),
        other => other,
    }
}

fn date_item(label: &str, value: DateTime<Utc>, update: &'static dyn Fn(&Obj, DateTime<Utc>)) -> ConfigEntry<Obj> {
    ConfigEntry::Item(
        ListItem::new(
            get_text(label),
            ListValue::Date(value, icy_board_tui::config_menu::DateEditState::from_date(&value)),
        )
        .with_status(get_text(label))
        .with_label_width(LABEL_WIDTH)
        .with_update_date_value(update),
    )
}

impl MaintenancePage {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, op: MaintenanceOp) -> Self {
        let mut params = Params::default();
        // The screens that ask for a range start at the one the utility used;
        // the screens without such a question must not filter at all.
        params.max_security = match op {
            MaintenanceOp::Pack | MaintenanceOp::StandardizePhones | MaintenanceOp::CopyExpiredSecurity | MaintenanceOp::InitializeCounters => 255,
            _ => 110,
        };

        let width = screen_label_width(op);
        let entry = match op {
            MaintenanceOp::Pack => vec![
                ConfigEntry::Separator,
                ConfigEntry::Group(
                    get_text("icbsm_pack_removal_group"),
                    vec![
                        wide(bool_item(
                            "icbsm_remove_deleted_or_locked",
                            params.remove_deleted_or_locked,
                            &|o: &Obj, v: bool| o.lock().unwrap().remove_deleted_or_locked = v,
                        )),
                        wide(with_hint(
                            u32_item("icbsm_inactive_days", params.inactive_days, 0, DAYS_OFF, &|o: &Obj, v: u32| {
                                o.lock().unwrap().inactive_days = v
                            }),
                            "icbsm_inactive_days-status",
                        )),
                        wide(with_hint(
                            date_item("icbsm_last_on_since", params.last_on_since, &|o: &Obj, v: DateTime<Utc>| {
                                o.lock().unwrap().last_on_since = v
                            }),
                            "icbsm_date_off-status",
                        )),
                        wide(with_hint(
                            date_item("icbsm_expired_before", params.expired_before, &|o: &Obj, v: DateTime<Utc>| {
                                o.lock().unwrap().expired_before = v
                            }),
                            "icbsm_date_off-status",
                        )),
                    ],
                ),
                ConfigEntry::Separator,
                ConfigEntry::Group(
                    get_text("icbsm_pack_keep_group"),
                    vec![
                        wide(with_hint(
                            u32_item("icbsm_keep_security", params.keep_security, 0, 255, &|o: &Obj, v: u32| {
                                o.lock().unwrap().keep_security = v
                            }),
                            "icbsm_keep_security-status",
                        )),
                        wide(bool_item("icbsm_keep_locked_out", params.keep_locked_out, &|o: &Obj, v: bool| {
                            o.lock().unwrap().keep_locked_out = v
                        })),
                    ],
                ),
            ],

            MaintenanceOp::AdjustSecurity | MaintenanceOp::AdjustSecurityExpired => vec![
                ConfigEntry::Separator,
                sized(
                    u32_item("icbsm_min_security", params.min_security, 0, 255, &|o: &Obj, v: u32| {
                        o.lock().unwrap().min_security = v
                    }),
                    width,
                ),
                continued(
                    u32_item("icbsm_max_security", params.max_security, 0, 255, &|o: &Obj, v: u32| {
                        o.lock().unwrap().max_security = v
                    }),
                    width,
                ),
                ConfigEntry::Separator,
                sized(
                    u32_item("icbsm_new_level", params.new_level, 0, 255, &|o: &Obj, v: u32| o.lock().unwrap().new_level = v),
                    27,
                ),
            ],

            MaintenanceOp::InitializeCounters => vec![
                ConfigEntry::Label(get_text("icbsm_counters_option1")),
                ConfigEntry::Label(get_text("icbsm_counters_option2")),
                ConfigEntry::Label(get_text("icbsm_counters_option3")),
                ConfigEntry::Label(get_text("icbsm_counters_option4")),
                ConfigEntry::Separator,
                sized(
                    u32_item("icbsm_counters_choose", params.counter_option, 1, 4, &|o: &Obj, v: u32| {
                        o.lock().unwrap().counter_option = v
                    }),
                    COUNTER_LABEL_WIDTH,
                ),
                ConfigEntry::Separator,
                sized(
                    bool_item("icbsm_counters_files", params.counter_files, &|o: &Obj, v: bool| {
                        o.lock().unwrap().counter_files = v
                    }),
                    COUNTER_LABEL_WIDTH,
                ),
                sized(
                    bool_item("icbsm_counters_bytes", params.counter_bytes, &|o: &Obj, v: bool| {
                        o.lock().unwrap().counter_bytes = v
                    }),
                    COUNTER_LABEL_WIDTH,
                ),
            ],

            MaintenanceOp::AdjustExpiration => vec![
                ConfigEntry::Separator,
                ConfigEntry::Group(
                    get_text("icbsm_expiration_range_group"),
                    vec![
                        sized(
                            bool_item("icbsm_use_expired_level", params.use_expired_level, &|o: &Obj, v: bool| {
                                o.lock().unwrap().use_expired_level = v
                            }),
                            width,
                        ),
                        sized(
                            u32_item("icbsm_exp_min_security", params.min_security, 0, 255, &|o: &Obj, v: u32| {
                                o.lock().unwrap().min_security = v
                            }),
                            width,
                        ),
                        sized(
                            u32_item("icbsm_exp_max_security", params.max_security, 0, 255, &|o: &Obj, v: u32| {
                                o.lock().unwrap().max_security = v
                            }),
                            width,
                        ),
                    ],
                ),
                ConfigEntry::Separator,
                ConfigEntry::Group(
                    get_text("icbsm_expiration_change_group"),
                    vec![
                        sized(
                            with_hint(
                                date_item("icbsm_expiration_date", params.expiration_date, &|o: &Obj, v: DateTime<Utc>| {
                                    o.lock().unwrap().expiration_date = v
                                }),
                                "icbsm_date_off-status",
                            ),
                            41,
                        ),
                        sized(
                            u32_item("icbsm_add_days", params.add_days, 0, 9999, &|o: &Obj, v: u32| o.lock().unwrap().add_days = v),
                            41,
                        ),
                    ],
                ),
            ],

            MaintenanceOp::ConferenceInsert | MaintenanceOp::ConferenceRemove => {
                let removing = op == MaintenanceOp::ConferenceRemove;
                vec![
                    ConfigEntry::Separator,
                    sized(
                        u32_item(
                            if removing { "icbsm_conf_first_remove" } else { "icbsm_conf_first_insert" },
                            params.conf_first,
                            0,
                            65535,
                            &|o: &Obj, v: u32| o.lock().unwrap().conf_first = v,
                        ),
                        width,
                    ),
                    sized(
                        u32_item(
                            if removing { "icbsm_conf_last_remove" } else { "icbsm_conf_last_insert" },
                            params.conf_last,
                            0,
                            65535,
                            &|o: &Obj, v: u32| o.lock().unwrap().conf_last = v,
                        ),
                        width,
                    ),
                    ConfigEntry::Separator,
                    sized(
                        bool_item("icbsm_flag_registered", params.flag_registered, &|o: &Obj, v: bool| {
                            o.lock().unwrap().flag_registered = v
                        }),
                        width,
                    ),
                    sized(
                        bool_item("icbsm_flag_expired", params.flag_expired, &|o: &Obj, v: bool| {
                            o.lock().unwrap().flag_expired = v
                        }),
                        width,
                    ),
                    sized(
                        bool_item("icbsm_flag_selected", params.flag_selected, &|o: &Obj, v: bool| {
                            o.lock().unwrap().flag_selected = v
                        }),
                        width,
                    ),
                    sized(
                        bool_item("icbsm_flag_sysop", params.flag_sysop, &|o: &Obj, v: bool| o.lock().unwrap().flag_sysop = v),
                        width,
                    ),
                    sized(
                        bool_item("icbsm_flag_net_status", params.flag_net_status, &|o: &Obj, v: bool| {
                            o.lock().unwrap().flag_net_status = v
                        }),
                        width,
                    ),
                    sized(
                        bool_item("icbsm_reset_lastread", params.reset_lastread, &|o: &Obj, v: bool| {
                            o.lock().unwrap().reset_lastread = v
                        }),
                        width,
                    ),
                    ConfigEntry::Separator,
                    sized(
                        u32_item("icbsm_conf_min_security", params.min_security, 0, 255, &|o: &Obj, v: u32| {
                            o.lock().unwrap().min_security = v
                        }),
                        width,
                    ),
                    continued(
                        u32_item("icbsm_conf_max_security", params.max_security, 0, 255, &|o: &Obj, v: u32| {
                            o.lock().unwrap().max_security = v
                        }),
                        width,
                    ),
                ]
            }

            MaintenanceOp::ConferenceMove => vec![
                ConfigEntry::Separator,
                sized(
                    u32_item("icbsm_conf_from", params.conf_first, 0, 65535, &|o: &Obj, v: u32| {
                        o.lock().unwrap().conf_first = v
                    }),
                    width,
                ),
                sized(
                    u32_item("icbsm_conf_to", params.conf_target, 0, 65535, &|o: &Obj, v: u32| {
                        o.lock().unwrap().conf_target = v
                    }),
                    width,
                ),
                ConfigEntry::Separator,
                sized(
                    bool_item("icbsm_move_flag_registered", params.flag_registered, &|o: &Obj, v: bool| {
                        o.lock().unwrap().flag_registered = v
                    }),
                    width,
                ),
                sized(
                    bool_item("icbsm_move_flag_expired", params.flag_expired, &|o: &Obj, v: bool| {
                        o.lock().unwrap().flag_expired = v
                    }),
                    width,
                ),
                sized(
                    bool_item("icbsm_move_flag_selected", params.flag_selected, &|o: &Obj, v: bool| {
                        o.lock().unwrap().flag_selected = v
                    }),
                    width,
                ),
                sized(
                    bool_item("icbsm_move_flag_sysop", params.flag_sysop, &|o: &Obj, v: bool| o.lock().unwrap().flag_sysop = v),
                    width,
                ),
                sized(
                    bool_item("icbsm_flag_net_status", params.flag_net_status, &|o: &Obj, v: bool| {
                        o.lock().unwrap().flag_net_status = v
                    }),
                    width,
                ),
                sized(
                    bool_item("icbsm_move_last_conference", params.move_last_conference, &|o: &Obj, v: bool| {
                        o.lock().unwrap().move_last_conference = v
                    }),
                    width,
                ),
                sized(
                    bool_item("icbsm_move_lastread", params.reset_lastread, &|o: &Obj, v: bool| {
                        o.lock().unwrap().reset_lastread = v
                    }),
                    width,
                ),
                ConfigEntry::Separator,
                sized(
                    u32_item("icbsm_move_min_security", params.min_security, 0, 255, &|o: &Obj, v: u32| {
                        o.lock().unwrap().min_security = v
                    }),
                    width,
                ),
                sized(
                    u32_item("icbsm_move_max_security", params.max_security, 0, 255, &|o: &Obj, v: u32| {
                        o.lock().unwrap().max_security = v
                    }),
                    width,
                ),
            ],

            MaintenanceOp::CopyExpiredSecurity | MaintenanceOp::StandardizePhones => Vec::new(),
        };

        Self {
            op,
            icy_board,
            menu: ConfigMenu {
                obj: Arc::new(Mutex::new(params)),
                entry,
            },
            state: ConfigMenuState::default(),
            stage: if matches!(op, MaintenanceOp::StandardizePhones | MaintenanceOp::CopyExpiredSecurity) {
                Stage::Confirm
            } else {
                Stage::Criteria
            },
            error: None,
        }
    }

    fn selection(&self) -> UserSelection {
        let p = self.menu.obj.lock().unwrap();
        let packing = self.op == MaintenanceOp::Pack;
        UserSelection {
            min_security: p.min_security.min(255) as u8,
            max_security: p.max_security.min(255) as u8,
            security_field: if p.use_expired_level || self.op == MaintenanceOp::AdjustSecurityExpired {
                SecurityField::Expired
            } else {
                SecurityField::Normal
            },
            last_on_before: (packing && date_is_set(p.last_on_since)).then_some(p.last_on_since),
            inactive_days: (packing && p.inactive_days < DAYS_OFF).then_some(p.inactive_days),
            never_logged_on: false,
            delete_flagged: packing && p.remove_deleted_or_locked,
            disabled: packing && p.remove_deleted_or_locked,
            locked_out: packing && p.remove_deleted_or_locked,
            expired_before: (packing && date_is_set(p.expired_before)).then_some(p.expired_before),
            keep_security_at_least: (packing && p.keep_security > 0).then_some(p.keep_security.min(255) as u8),
            keep_locked_out: packing && p.keep_locked_out,
            protect_first_record: packing,
            protected_names: Vec::new(),
        }
    }

    fn conference_flags(&self) -> ConferenceFlags {
        let p = self.menu.obj.lock().unwrap();
        let mut flags = ConferenceFlags::None;
        if p.flag_registered {
            flags |= ConferenceFlags::Registered;
        }
        if p.flag_expired {
            flags |= ConferenceFlags::Expired;
        }
        if p.flag_selected {
            flags |= ConferenceFlags::Selected;
        }
        if p.flag_sysop {
            flags |= ConferenceFlags::Sysop;
        }
        if p.flag_net_status {
            flags |= ConferenceFlags::NetStatus;
        }
        flags
    }

    /// Numbers above the last conference would only set flags nobody reads.
    fn last_conference(&self) -> usize {
        self.icy_board.lock().unwrap().conferences.len().saturating_sub(1)
    }

    fn conference_range(&self) -> Vec<usize> {
        let highest = self.last_conference();
        let p = self.menu.obj.lock().unwrap();
        let first = (p.conf_first as usize).min(highest);
        let last = (p.conf_last.max(p.conf_first) as usize).min(highest);
        (first..=last).collect()
    }

    fn build_preview(&mut self) {
        let board = self.icy_board.lock().unwrap();
        let selected = self.selection().select(&board.users, Utc::now());
        let names = selected.iter().map(|i| board.users[*i].get_name().clone()).collect();
        let matched = selected.len();
        drop(board);
        self.stage = Stage::Preview { matched, names };
    }

    fn run(&mut self) {
        let now = Utc::now();
        let selection = self.selection();
        let flags = self.conference_flags();
        let conferences = self.conference_range();
        let highest_conference = self.last_conference();
        let (target, from, to, reset_lastread, move_last_conference, new_level, counters, change) = {
            let p = self.menu.obj.lock().unwrap();
            (
                if self.op == MaintenanceOp::AdjustSecurityExpired {
                    SecurityField::Expired
                } else {
                    SecurityField::Normal
                },
                (p.conf_first as usize).min(highest_conference),
                (p.conf_target as usize).min(highest_conference),
                p.reset_lastread,
                p.move_last_conference,
                p.new_level.min(255) as u8,
                (counter_init_from_option(p.counter_option), counter_scope(p.counter_files, p.counter_bytes)),
                if date_is_set(p.expiration_date) {
                    ExpirationChange::SetDate(p.expiration_date)
                } else {
                    ExpirationChange::AddDays(p.add_days as i64)
                },
            )
        };

        let mut board = self.icy_board.lock().unwrap();
        let users_file = board.resolve_file(&board.config.paths.user_file);
        if let Err(err) = user_maintenance::create_backup(&users_file) {
            self.error = Some(get_text_args("icbsm_backup_failed", HashMap::from([("error".to_string(), err.to_string())])));
            return;
        }

        let original = board.users.clone();
        let report = match self.op {
            MaintenanceOp::Pack => user_maintenance::pack(&mut board.users, &selection, now),
            MaintenanceOp::AdjustSecurity | MaintenanceOp::AdjustSecurityExpired => {
                user_maintenance::adjust_security(&mut board.users, &selection, new_level, target, now)
            }
            MaintenanceOp::CopyExpiredSecurity => user_maintenance::copy_expired_security(&mut board.users, &selection, now),
            MaintenanceOp::InitializeCounters => user_maintenance::initialize_counters(&mut board.users, &selection, counters.0, counters.1, now),
            MaintenanceOp::AdjustExpiration => user_maintenance::adjust_expiration(&mut board.users, &selection, change, now),
            MaintenanceOp::ConferenceInsert => user_maintenance::conference_register(&mut board.users, &selection, &conferences, flags, reset_lastread, now),
            MaintenanceOp::ConferenceRemove => user_maintenance::conference_unregister(&mut board.users, &selection, &conferences, flags, reset_lastread, now),
            MaintenanceOp::ConferenceMove => {
                user_maintenance::conference_move(&mut board.users, &selection, from, to, flags, reset_lastread, move_last_conference, now)
            }
            MaintenanceOp::StandardizePhones => user_maintenance::standardize_phones(&mut board.users, &selection, now),
        };

        if let Err(err) = board.save_userbase() {
            board.users = original;
            self.error = Some(get_text_args("icbsm_save_failed", HashMap::from([("error".to_string(), err.to_string())])));
            return;
        }
        drop(board);
        self.stage = Stage::Done { report };
    }

    fn render_lines(&self, frame: &mut Frame, area: Rect, title: String, bottom: String, lines: Vec<Line<'static>>) {
        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 0))
            .title_alignment(Alignment::Center)
            .title(Span::styled(title, get_tui_theme().dialog_box_title))
            .title_bottom(Span::styled(bottom, get_tui_theme().key_binding));

        Paragraph::new(Text::from(lines))
            .style(get_tui_theme().item)
            .wrap(Wrap { trim: false })
            .block(block)
            .render(area, frame.buffer_mut());
    }
}

impl Page for MaintenancePage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        // A run that could not write says so whatever stage it stopped in.
        if let Some(error) = self.error.clone() {
            self.render_lines(frame, area, self.op.title(), get_text("icbsm_done_keys"), vec![Line::from(error)]);
            return;
        }

        match &self.stage {
            Stage::Confirm => {
                super::render_question(frame, disp_area, &get_text("icbsm_are_you_sure"), &get_text("icbsm_question_keys"));
            }
            Stage::Criteria => {
                let block = Block::new()
                    .style(get_tui_theme().background)
                    .borders(Borders::ALL)
                    .border_set(BORDER_SET)
                    .border_style(get_tui_theme().dialog_box)
                    .padding(Padding::new(2, 2, 1, 0))
                    .title_alignment(Alignment::Center)
                    .title(Span::styled(self.op.title(), get_tui_theme().dialog_box_title))
                    .title_bottom(Span::styled(get_text("icbsm_criteria_keys"), get_tui_theme().key_binding));
                block.render(area, frame.buffer_mut());

                let inner = area.inner(Margin { vertical: 1, horizontal: 2 });
                self.menu.render(inner, frame, &mut self.state);
            }
            Stage::Preview { matched, names } => {
                let mut lines = vec![
                    Line::from(get_text_args(
                        "icbsm_preview_count",
                        HashMap::from([("count".to_string(), matched.to_string())]),
                    )),
                    Line::from(""),
                ];
                let room = area.height.saturating_sub(6) as usize;
                for name in names.iter().take(room) {
                    lines.push(Line::from(format!("  {name}")));
                }
                if names.len() > room {
                    lines.push(Line::from(get_text_args(
                        "icbsm_preview_more",
                        HashMap::from([("count".to_string(), (names.len() - room).to_string())]),
                    )));
                }
                if self.op.removes_users() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(get_text("icbsm_preview_pack_warning")));
                }
                self.render_lines(frame, area, self.op.title(), get_text("icbsm_preview_keys"), lines);
            }
            Stage::Done { report } => {
                let mut lines = vec![Line::from(get_text_args(
                    "icbsm_done_count",
                    HashMap::from([
                        ("changed".to_string(), report.changed.to_string()),
                        ("matched".to_string(), report.matched.to_string()),
                    ]),
                ))];
                if let Some(error) = &self.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(error.clone()));
                } else {
                    lines.push(Line::from(""));
                    lines.push(Line::from(get_text("icbsm_done_backup_hint")));
                }
                self.render_lines(frame, area, self.op.title(), get_text("icbsm_done_keys"), lines);
            }
        }

        if let Some(error) = &self.error {
            if !matches!(self.stage, Stage::Done { .. }) {
                let lines = vec![Line::from(error.clone())];
                self.render_lines(frame, area, self.op.title(), get_text("icbsm_done_keys"), lines);
            }
        }
    }

    fn request_status(&self) -> ResultState {
        match self.stage {
            Stage::Criteria => ResultState {
                edit_msg: EditMessage::None,
                status_line: self.menu.current_status_line(&self.state),
            },
            _ => ResultState::default(),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.error.is_some() {
            self.error = None;
            return PageMessage::Close;
        }

        match &self.stage {
            Stage::Confirm => match key.code {
                KeyCode::Esc => PageMessage::Close,
                KeyCode::Enter | KeyCode::PageDown | KeyCode::F(2) => {
                    self.run();
                    PageMessage::None
                }
                _ => PageMessage::None,
            },
            Stage::Criteria => {
                if key.code == KeyCode::F(2) || key.code == KeyCode::PageDown {
                    self.build_preview();
                    return PageMessage::None;
                }
                let res = self.menu.handle_key_press(key, &mut self.state);
                if res.edit_msg == EditMessage::Close {
                    return PageMessage::Close;
                }
                PageMessage::ResultState(res)
            }
            Stage::Preview { matched, .. } => match key.code {
                KeyCode::Esc => {
                    self.stage = Stage::Criteria;
                    PageMessage::None
                }
                KeyCode::Enter | KeyCode::F(2) | KeyCode::PageDown => {
                    if *matched > 0 {
                        self.run();
                    } else {
                        self.stage = Stage::Criteria;
                    }
                    PageMessage::None
                }
                _ => PageMessage::None,
            },
            Stage::Done { .. } => PageMessage::Close,
        }
    }
}

/// Puts the user file back the way it was before the last destructive run.
pub struct UndoPage {
    icy_board: Arc<Mutex<IcyBoard>>,
    message: Option<String>,
}

impl UndoPage {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self { icy_board, message: None }
    }

    fn users_file(&self) -> std::path::PathBuf {
        let board = self.icy_board.lock().unwrap();
        board.resolve_file(&board.config.paths.user_file)
    }

    fn restore(&mut self) {
        let users_file = self.users_file();
        if let Err(err) = user_maintenance::restore_backup(&users_file) {
            self.message = Some(get_text_args("icbsm_undo_failed", HashMap::from([("error".to_string(), err.to_string())])));
            return;
        }
        match icy_board_engine::icy_board::IcyBoardSerializer::load(&users_file) {
            Ok(users) => {
                self.icy_board.lock().unwrap().users = users;
                self.message = Some(get_text("icbsm_undo_done"));
            }
            Err(err) => {
                self.message = Some(get_text_args("icbsm_undo_failed", HashMap::from([("error".to_string(), err.to_string())])));
            }
        }
    }
}

impl Page for UndoPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let users_file = self.users_file();
        let mut lines = Vec::new();
        let bottom = if let Some(message) = &self.message {
            lines.push(Line::from(message.clone()));
            get_text("icbsm_done_keys")
        } else if let Some(time) = user_maintenance::backup_time(&users_file) {
            lines.push(Line::from(get_text_args(
                "icbsm_undo_prompt",
                HashMap::from([("date".to_string(), time.format("%Y-%m-%d %H:%M:%S").to_string())]),
            )));
            get_text("icbsm_undo_keys")
        } else {
            lines.push(Line::from(get_text("icbsm_undo_no_backup")));
            get_text("icbsm_done_keys")
        };

        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 0))
            .title_alignment(Alignment::Center)
            .title(Span::styled(get_text("icbsm_undo_title"), get_tui_theme().dialog_box_title))
            .title_bottom(Span::styled(bottom, get_tui_theme().key_binding));

        Paragraph::new(Text::from(lines))
            .style(get_tui_theme().item)
            .wrap(Wrap { trim: false })
            .block(block)
            .render(area, frame.buffer_mut());
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.message.is_some() {
            return PageMessage::Close;
        }
        match key.code {
            KeyCode::Enter if user_maintenance::has_backup(&self.users_file()) => {
                self.restore();
                PageMessage::None
            }
            KeyCode::Esc => PageMessage::Close,
            _ => PageMessage::None,
        }
    }
}
