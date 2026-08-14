use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
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

/// The bulk operations offered below "Users File Maintenance".
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceOp {
    Pack,
    AdjustSecurity,
    CopyExpiredSecurity,
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
            MaintenanceOp::AdjustSecurity => get_text("icbsm_adjust_security_title"),
            MaintenanceOp::CopyExpiredSecurity => get_text("icbsm_copy_expired_title"),
            MaintenanceOp::AdjustExpiration => get_text("icbsm_adjust_expiration_title"),
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

    delete_flagged: bool,
    disabled: bool,
    never_logged_on: bool,
    inactive_days: u32,
    use_subscription_date: bool,
    subscription_date: DateTime<Utc>,
    keep_security: u32,
    keep_locked_out: bool,

    new_level: u32,
    write_expired_level: bool,

    set_expiration_date: bool,
    expiration_date: DateTime<Utc>,
    add_days: u32,

    conf_first: u32,
    conf_last: u32,
    conf_target: u32,
    flag_registered: bool,
    flag_expired: bool,
    flag_selected: bool,
    flag_sysop: bool,
    reset_lastread: bool,
    move_last_conference: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            min_security: 0,
            max_security: 255,
            use_expired_level: false,
            delete_flagged: true,
            disabled: false,
            never_logged_on: false,
            inactive_days: 0,
            use_subscription_date: false,
            subscription_date: Utc::now(),
            keep_security: 0,
            keep_locked_out: true,
            new_level: 10,
            write_expired_level: false,
            set_expiration_date: false,
            expiration_date: Utc::now(),
            add_days: 0,
            conf_first: 0,
            conf_last: 0,
            conf_target: 0,
            flag_registered: true,
            flag_expired: false,
            flag_selected: false,
            flag_sysop: false,
            reset_lastread: false,
            move_last_conference: false,
        }
    }
}

enum Stage {
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
        let params = Params::default();
        let mut entry = vec![
            ConfigEntry::Separator,
            u32_item("icbsm_min_security", params.min_security, 0, 255, &|o: &Obj, v: u32| {
                o.lock().unwrap().min_security = v
            }),
            u32_item("icbsm_max_security", params.max_security, 0, 255, &|o: &Obj, v: u32| {
                o.lock().unwrap().max_security = v
            }),
            bool_item("icbsm_use_expired_level", params.use_expired_level, &|o: &Obj, v: bool| {
                o.lock().unwrap().use_expired_level = v
            }),
        ];

        match op {
            MaintenanceOp::Pack => {
                entry.push(ConfigEntry::Separator);
                entry.push(bool_item("icbsm_delete_flagged", params.delete_flagged, &|o: &Obj, v: bool| {
                    o.lock().unwrap().delete_flagged = v
                }));
                entry.push(bool_item("icbsm_disabled", params.disabled, &|o: &Obj, v: bool| o.lock().unwrap().disabled = v));
                entry.push(bool_item("icbsm_never_logged_on", params.never_logged_on, &|o: &Obj, v: bool| {
                    o.lock().unwrap().never_logged_on = v
                }));
                entry.push(u32_item("icbsm_inactive_days", params.inactive_days, 0, 9999, &|o: &Obj, v: u32| {
                    o.lock().unwrap().inactive_days = v
                }));
                entry.push(bool_item("icbsm_use_subscription", params.use_subscription_date, &|o: &Obj, v: bool| {
                    o.lock().unwrap().use_subscription_date = v
                }));
                entry.push(date_item("icbsm_subscription_date", params.subscription_date, &|o: &Obj, v: DateTime<Utc>| {
                    o.lock().unwrap().subscription_date = v
                }));
                entry.push(ConfigEntry::Separator);
                entry.push(u32_item("icbsm_keep_security", params.keep_security, 0, 255, &|o: &Obj, v: u32| {
                    o.lock().unwrap().keep_security = v
                }));
                entry.push(bool_item("icbsm_keep_locked_out", params.keep_locked_out, &|o: &Obj, v: bool| {
                    o.lock().unwrap().keep_locked_out = v
                }));
            }
            MaintenanceOp::AdjustSecurity => {
                entry.push(ConfigEntry::Separator);
                entry.push(u32_item("icbsm_new_level", params.new_level, 0, 255, &|o: &Obj, v: u32| {
                    o.lock().unwrap().new_level = v
                }));
                entry.push(bool_item("icbsm_write_expired_level", params.write_expired_level, &|o: &Obj, v: bool| {
                    o.lock().unwrap().write_expired_level = v
                }));
            }
            MaintenanceOp::AdjustExpiration => {
                entry.push(ConfigEntry::Separator);
                entry.push(bool_item("icbsm_set_expiration", params.set_expiration_date, &|o: &Obj, v: bool| {
                    o.lock().unwrap().set_expiration_date = v
                }));
                entry.push(date_item("icbsm_expiration_date", params.expiration_date, &|o: &Obj, v: DateTime<Utc>| {
                    o.lock().unwrap().expiration_date = v
                }));
                entry.push(u32_item("icbsm_add_days", params.add_days, 0, 9999, &|o: &Obj, v: u32| {
                    o.lock().unwrap().add_days = v
                }));
            }
            MaintenanceOp::ConferenceInsert | MaintenanceOp::ConferenceRemove | MaintenanceOp::ConferenceMove => {
                entry.push(ConfigEntry::Separator);
                if op == MaintenanceOp::ConferenceMove {
                    entry.push(u32_item("icbsm_conf_from", params.conf_first, 0, 65535, &|o: &Obj, v: u32| {
                        o.lock().unwrap().conf_first = v
                    }));
                    entry.push(u32_item("icbsm_conf_to", params.conf_target, 0, 65535, &|o: &Obj, v: u32| {
                        o.lock().unwrap().conf_target = v
                    }));
                } else {
                    entry.push(u32_item("icbsm_conf_first", params.conf_first, 0, 65535, &|o: &Obj, v: u32| {
                        o.lock().unwrap().conf_first = v
                    }));
                    entry.push(u32_item("icbsm_conf_last", params.conf_last, 0, 65535, &|o: &Obj, v: u32| {
                        o.lock().unwrap().conf_last = v
                    }));
                }
                entry.push(bool_item("icbsm_flag_registered", params.flag_registered, &|o: &Obj, v: bool| {
                    o.lock().unwrap().flag_registered = v
                }));
                entry.push(bool_item("icbsm_flag_expired", params.flag_expired, &|o: &Obj, v: bool| {
                    o.lock().unwrap().flag_expired = v
                }));
                entry.push(bool_item("icbsm_flag_selected", params.flag_selected, &|o: &Obj, v: bool| {
                    o.lock().unwrap().flag_selected = v
                }));
                entry.push(bool_item("icbsm_flag_sysop", params.flag_sysop, &|o: &Obj, v: bool| {
                    o.lock().unwrap().flag_sysop = v
                }));
                if op == MaintenanceOp::ConferenceMove {
                    entry.push(bool_item("icbsm_move_lastread", params.reset_lastread, &|o: &Obj, v: bool| {
                        o.lock().unwrap().reset_lastread = v
                    }));
                    entry.push(bool_item("icbsm_move_last_conference", params.move_last_conference, &|o: &Obj, v: bool| {
                        o.lock().unwrap().move_last_conference = v
                    }));
                } else {
                    entry.push(bool_item("icbsm_reset_lastread", params.reset_lastread, &|o: &Obj, v: bool| {
                        o.lock().unwrap().reset_lastread = v
                    }));
                }
            }
            MaintenanceOp::CopyExpiredSecurity | MaintenanceOp::StandardizePhones => {}
        }

        Self {
            op,
            icy_board,
            menu: ConfigMenu {
                obj: Arc::new(Mutex::new(params)),
                entry,
            },
            state: ConfigMenuState::default(),
            stage: Stage::Criteria,
            error: None,
        }
    }

    fn selection(&self) -> UserSelection {
        let p = self.menu.obj.lock().unwrap();
        UserSelection {
            min_security: p.min_security.min(255) as u8,
            max_security: p.max_security.min(255) as u8,
            security_field: if p.use_expired_level { SecurityField::Expired } else { SecurityField::Normal },
            last_on_before: None,
            inactive_days: (self.op == MaintenanceOp::Pack && p.inactive_days > 0).then_some(p.inactive_days),
            never_logged_on: self.op == MaintenanceOp::Pack && p.never_logged_on,
            delete_flagged: self.op == MaintenanceOp::Pack && p.delete_flagged,
            disabled: self.op == MaintenanceOp::Pack && p.disabled,
            expired_before: (self.op == MaintenanceOp::Pack && p.use_subscription_date).then_some(p.subscription_date),
            keep_security_at_least: (p.keep_security > 0).then_some(p.keep_security.min(255) as u8),
            keep_locked_out: self.op == MaintenanceOp::Pack && p.keep_locked_out,
            protect_first_record: true,
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
        flags
    }

    fn conference_range(&self) -> Vec<usize> {
        let p = self.menu.obj.lock().unwrap();
        let first = p.conf_first as usize;
        let last = p.conf_last.max(p.conf_first) as usize;
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
        let (target, from, to, reset_lastread, move_last_conference, new_level, write_expired, change) = {
            let p = self.menu.obj.lock().unwrap();
            (
                if p.write_expired_level {
                    SecurityField::Expired
                } else {
                    SecurityField::Normal
                },
                p.conf_first as usize,
                p.conf_target as usize,
                p.reset_lastread,
                p.move_last_conference,
                p.new_level.min(255) as u8,
                p.write_expired_level,
                if p.set_expiration_date {
                    ExpirationChange::SetDate(p.expiration_date)
                } else {
                    ExpirationChange::AddDays(p.add_days as i64)
                },
            )
        };
        let _ = write_expired;

        let mut board = self.icy_board.lock().unwrap();
        let users_file = board.resolve_file(&board.config.paths.user_file);
        if let Err(err) = user_maintenance::create_backup(&users_file) {
            self.error = Some(get_text_args("icbsm_backup_failed", HashMap::from([("error".to_string(), err.to_string())])));
            return;
        }

        let report = match self.op {
            MaintenanceOp::Pack => user_maintenance::pack(&mut board.users, &selection, now),
            MaintenanceOp::AdjustSecurity => user_maintenance::adjust_security(&mut board.users, &selection, new_level, target, now),
            MaintenanceOp::CopyExpiredSecurity => user_maintenance::copy_expired_security(&mut board.users, &selection, now),
            MaintenanceOp::AdjustExpiration => user_maintenance::adjust_expiration(&mut board.users, &selection, change, now),
            MaintenanceOp::ConferenceInsert => user_maintenance::conference_register(&mut board.users, &selection, &conferences, flags, reset_lastread, now),
            MaintenanceOp::ConferenceRemove => user_maintenance::conference_unregister(&mut board.users, &selection, &conferences, flags, reset_lastread, now),
            MaintenanceOp::ConferenceMove => {
                user_maintenance::conference_move(&mut board.users, &selection, from, to, flags, reset_lastread, move_last_conference, now)
            }
            MaintenanceOp::StandardizePhones => user_maintenance::standardize_phones(&mut board.users, &selection, now),
        };

        if let Err(err) = board.save_userbase() {
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
            .title(title)
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

        match &self.stage {
            Stage::Criteria => {
                let block = Block::new()
                    .style(get_tui_theme().background)
                    .borders(Borders::ALL)
                    .border_set(BORDER_SET)
                    .border_style(get_tui_theme().dialog_box)
                    .padding(Padding::new(2, 2, 1, 0))
                    .title_alignment(Alignment::Center)
                    .title(self.op.title())
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
            Stage::Criteria => {
                if key.code == KeyCode::F(2) {
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
                KeyCode::Enter | KeyCode::F(2) => {
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
            .title(get_text("icbsm_undo_title"))
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
