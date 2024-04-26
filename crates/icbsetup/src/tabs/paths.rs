use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tui_textarea::TextArea;

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    theme::THEME,
};
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

use super::TabPage;

pub struct PathTab<'a> {
    pub state: ConfigMenuState,
    config: ConfigMenu,
    icy_board: Arc<Mutex<IcyBoard>>,

    edit_text: Option<PathBuf>,
    textarea: TextArea<'a>,
}

impl<'a> PathTab<'a> {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let a = icy_board.clone();
        let lock = a.lock().unwrap();
        let system_files = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "conf_data",
                    "Conference Data".to_string(),
                    ListValue::Path(lock.config.paths.conferences.clone()),
                )
                .with_status("Name/Loc of Conference Data"),
            ),
            ConfigEntry::Item(
                ListItem::new("home_dir", "Home Directory".to_string(), ListValue::Path(lock.config.paths.home_dir.clone())).with_status("User Home Directory"),
            ),
            ConfigEntry::Item(
                ListItem::new("log_file", "Log File".to_string(), ListValue::Path(lock.config.paths.log_file.clone())).with_status("BBS Logfile"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "stats_file",
                    "Statistics File".to_string(),
                    ListValue::Path(lock.config.paths.statistics_file.clone()),
                )
                .with_status("Name/Loc of Statistics file"),
            ),
            ConfigEntry::Item(
                ListItem::new("icb_text", "ICBTEXT File".to_string(), ListValue::Path(lock.config.paths.icbtext.clone()))
                    .with_status("Name/Loc of ICBTEXT file"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "temp_files",
                    "Temporary Work Files".to_string(),
                    ListValue::Path(lock.config.paths.tmp_work_path.clone()),
                )
                .with_status("Location of Temporary Work Files"),
            ),
            ConfigEntry::Item(
                ListItem::new("help_files", "Help Files".to_string(), ListValue::Path(lock.config.paths.help_path.clone()))
                    .with_status("Location of Help Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "security_file_path",
                    "Login Security Files".to_string(),
                    ListValue::Path(lock.config.paths.security_file_path.clone()),
                )
                .with_status("Location of Login Security Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "command_display_path",
                    "Command Display Files".to_string(),
                    ListValue::Path(lock.config.paths.command_display_path.clone()),
                )
                .with_status("Location of Command Display Files"),
            ),
        ];

        let configuration_files: Vec<ConfigEntry> = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec_level_file",
                    "PWRD/Security File".to_string(),
                    ListValue::Path(lock.config.paths.pwrd_sec_level_file.clone()),
                )
                .with_status("Name/Location of PWRD/Security File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "trashcan_user",
                    "User Trashcan File".to_string(),
                    ListValue::Path(lock.config.paths.trashcan_user.clone()),
                )
                .with_status("Name/Location of User Trashcan File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "trashcan_passwords",
                    "Password Trashcan File".to_string(),
                    ListValue::Path(lock.config.paths.trashcan_passwords.clone()),
                )
                .with_status("Name/Location of Password Trashcan File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "trashcan_email",
                    "EMail Trashcan File".to_string(),
                    ListValue::Path(lock.config.paths.trashcan_email.clone()),
                )
                .with_status("Name/Location of EMail Trashcan File"),
            ),
            ConfigEntry::Item(
                ListItem::new("vip_users", "VIP Users File".to_string(), ListValue::Path(lock.config.paths.vip_users.clone()))
                    .with_status("Name/Location of VIP Users File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "protocol_data_file",
                    "Protocol Data File".to_string(),
                    ListValue::Path(lock.config.paths.protocol_data_file.clone()),
                )
                .with_status("Name/Location of Protocol Data File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "language_file",
                    "Multi-Lang. Data File".to_string(),
                    ListValue::Path(lock.config.paths.language_file.clone()),
                )
                .with_status("Name/Location of Multi-Lang. Data File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "language_file",
                    "Default CMD.LST File".to_string(),
                    ListValue::Path(lock.config.paths.language_file.clone()),
                )
                .with_status("Name/Location of CMD.LST File"),
            ),
        ];

        let display_files = vec![
            ConfigEntry::Item(
                ListItem::new("welcome", "WELCOME File".to_string(), ListValue::Path(lock.config.paths.welcome.clone()))
                    .with_status("Name/Location of WELCOME File"),
            ),
            ConfigEntry::Item(
                ListItem::new("newuser", "NEWUSER File".to_string(), ListValue::Path(lock.config.paths.newuser.clone()))
                    .with_status("Name/Location of NEWUSER File"),
            ),
            ConfigEntry::Item(
                ListItem::new("closed", "CLOSED File".to_string(), ListValue::Path(lock.config.paths.closed.clone()))
                    .with_status("Name/Location of CLOSED File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "expire_warning",
                    "WARNING File".to_string(),
                    ListValue::Path(lock.config.paths.expire_warning.clone()),
                )
                .with_status("Name/Location of WARNING File"),
            ),
            ConfigEntry::Item(
                ListItem::new("expired", "EXPIRED File".to_string(), ListValue::Path(lock.config.paths.expired.clone()))
                    .with_status("Name/Location of EXPIRED File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "conf_join_menu",
                    "Conference Join Menu".to_string(),
                    ListValue::Path(lock.config.paths.conf_join_menu.clone()),
                )
                .with_status("Name/Location of Conference Join Menu File"),
            ),
            ConfigEntry::Item(
                ListItem::new("no_ansi", "NOANSI Warning".to_string(), ListValue::Path(lock.config.paths.no_ansi.clone()))
                    .with_status("Name/Location of NOANSI Warning File"),
            ),
        ];

        let new_user_files = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "newask_survey",
                    "New Reg Survey".to_string(),
                    ListValue::Path(lock.config.paths.newask_survey.clone()),
                )
                .with_status("Name/Location of NEWASK Survey File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "newask_answer",
                    "New Reg Answers".to_string(),
                    ListValue::Path(lock.config.paths.newask_answer.clone()),
                )
                .with_status("Name/Location of NEWASK Survey Answers"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "logon_survey",
                    "Logon Survey".to_string(),
                    ListValue::Path(lock.config.paths.logon_survey.clone()),
                )
                .with_status("Name/Location of Logon Survey File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "logon_answer",
                    "Logon Answers".to_string(),
                    ListValue::Path(lock.config.paths.logon_answer.clone()),
                )
                .with_status("Name/Location of Logon Survey Answers"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "logoff_survey",
                    "Logoff Survey".to_string(),
                    ListValue::Path(lock.config.paths.logoff_survey.clone()),
                )
                .with_status("Name/Location of Logoff Survey File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "logoff_answer",
                    "Logoff Answers".to_string(),
                    ListValue::Path(lock.config.paths.logoff_answer.clone()),
                )
                .with_status("Name/Location of Logoff Survey Answers"),
            ),
        ];

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                items: vec![
                    ConfigEntry::Group("System Files & Directories".to_string(), system_files),
                    ConfigEntry::Group("Configuration Files".to_string(), configuration_files),
                    ConfigEntry::Group("Display Files".to_string(), display_files),
                    ConfigEntry::Group("New User/Logon/off Surveys".to_string(), new_user_files),
                ],
            },
            icy_board,
            edit_text: None,
            textarea: TextArea::default(),
        }
    }

    fn write_back(&self, icy_board: &mut IcyBoard) {
        for entry in self.config.items.iter() {
            self.visit_item(&entry, icy_board);
        }
    }

    fn visit_item(&self, entry: &ConfigEntry, icy_board: &mut IcyBoard) {
        match entry {
            ConfigEntry::Group(_grp, entries) => {
                for e in entries {
                    self.visit_item(&e, icy_board);
                }
            }
            ConfigEntry::Separator => {}
            ConfigEntry::Item(item) => self.write_item(&item, icy_board),
            ConfigEntry::Table(_, _) => todo!(),
        }
    }

    fn write_item(&self, item: &ListItem, icy_board: &mut IcyBoard) {
        match &item.value {
            ListValue::Text(_, _text) => match item.id.as_str() {
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Path(path) => match item.id.as_str() {
                "conf_data" => icy_board.config.paths.conferences = path.clone(),
                "home_dir" => icy_board.config.paths.home_dir = path.clone(),
                "log_file" => icy_board.config.paths.log_file = path.clone(),
                "stats_file" => icy_board.config.paths.statistics_file = path.clone(),
                "icb_text" => icy_board.config.paths.icbtext = path.clone(),
                "temp_files" => icy_board.config.paths.tmp_work_path = path.clone(),
                "help_files" => icy_board.config.paths.help_path = path.clone(),
                "security_file_path" => icy_board.config.paths.security_file_path = path.clone(),
                "command_display_path" => icy_board.config.paths.command_display_path = path.clone(),
                "pwrd_sec_level_file" => icy_board.config.paths.pwrd_sec_level_file = path.clone(),
                "trashcan_user" => icy_board.config.paths.trashcan_user = path.clone(),
                "trashcan_passwords" => icy_board.config.paths.trashcan_passwords = path.clone(),
                "trashcan_email" => icy_board.config.paths.trashcan_email = path.clone(),
                "vip_users" => icy_board.config.paths.vip_users = path.clone(),
                "protocol_data_file" => icy_board.config.paths.protocol_data_file = path.clone(),
                "language_file" => icy_board.config.paths.language_file = path.clone(),
                "welcome" => icy_board.config.paths.welcome = path.clone(),
                "newuser" => icy_board.config.paths.newuser = path.clone(),
                "closed" => icy_board.config.paths.closed = path.clone(),
                "expire_warning" => icy_board.config.paths.expire_warning = path.clone(),
                "expired" => icy_board.config.paths.expired = path.clone(),
                "conf_join_menu" => icy_board.config.paths.conf_join_menu = path.clone(),

                "no_ansi" => icy_board.config.paths.no_ansi = path.clone(),
                "newask_survey" => icy_board.config.paths.newask_survey = path.clone(),
                "newask_answer" => icy_board.config.paths.newask_answer = path.clone(),
                "logon_survey" => icy_board.config.paths.logon_survey = path.clone(),
                "logon_answer" => icy_board.config.paths.logon_answer = path.clone(),
                "logoff_survey" => icy_board.config.paths.logoff_survey = path.clone(),
                "logoff_answer" => icy_board.config.paths.logoff_answer = path.clone(),

                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::U32(_i, _, _) => match item.id.as_str() {
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Bool(_b) => match item.id.as_str() {
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Color(_c) => match item.id.as_str() {
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::ValueList(_, _) => todo!(),
        }
    }
}

impl<'a> TabPage for PathTab<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(&Margin { horizontal: 2, vertical: 1 });
        Clear.render(area, frame.buffer_mut());

        if self.edit_text.is_some() {
            let block = Block::new()
                .title(format!("[{:?}]", self.edit_text.as_ref().unwrap()))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            block.render(area, frame.buffer_mut());
            let area = area.inner(&Margin { vertical: 1, horizontal: 1 });
            let widget = self.textarea.widget();
            frame.render_widget(widget, area);
            return;
        }

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(&Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame, &mut self.state);
    }
    fn has_control(&self) -> bool {
        self.state.in_edit || self.edit_text.is_some()
    }

    fn set_cursor_position(&self, frame: &mut Frame) {
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if self.edit_text.is_some() {
            if key.code == crossterm::event::KeyCode::Esc {
                self.edit_text = None;
                return ResultState::default();
            }
            self.textarea.input(key);
            return ResultState::default();
        }
        if !self.state.in_edit {
            match key.code {
                crossterm::event::KeyCode::F(2) => {
                    if let ListValue::Path(path) = &self.config.get_item(self.state.selected).unwrap().value {
                        let path = self.icy_board.lock().unwrap().resolve_file(path);

                        if path.exists() {
                            self.edit_text = Some(path.clone());
                            let text = fs::read_to_string(path).unwrap();
                            self.textarea = TextArea::new(text.lines().map(str::to_string).collect());
                        }
                    }
                }
                _ => {}
            }
        }

        let res = self.config.handle_key_press(key, &mut self.state);
        if self.state.in_edit {
            self.write_back(&mut self.icy_board.lock().unwrap());
        }
        self.state.in_edit = res.in_edit_mode;
        res
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            in_edit_mode: self.state.in_edit,
            status_line: if self.state.selected < self.config.items.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }
}
