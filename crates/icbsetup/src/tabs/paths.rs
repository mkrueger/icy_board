use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    theme::THEME,
    TerminalType,
};
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

use super::TabPage;

pub struct PathTab {
    pub state: ConfigMenuState,
    config: ConfigMenu,
}

impl PathTab {
    pub fn new(icy_board: Arc<IcyBoard>) -> Self {
        let system_files = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "conf_data",
                    "Conference Data".to_string(),
                    ListValue::Path(25, icy_board.config.paths.conferences.clone()),
                )
                .with_status("Name/Loc of Conference Data"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "home_dir",
                    "Home Directory".to_string(),
                    ListValue::Path(25, icy_board.config.paths.home_dir.clone()),
                )
                .with_status("User Home Directory"),
            ),
            ConfigEntry::Item(
                ListItem::new("log_file", "Log File".to_string(), ListValue::Path(25, icy_board.config.paths.log_file.clone())).with_status("BBS Logfile"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "stats_file",
                    "Statistics File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.statistics_file.clone()),
                )
                .with_status("Name/Loc of Statistics file"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "icb_text",
                    "ICBTEXT File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.icbtext.clone()),
                )
                .with_status("Name/Loc of ICBTEXT file"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "temp_files",
                    "Temporary Work Files".to_string(),
                    ListValue::Path(25, icy_board.config.paths.tmp_work_path.clone()),
                )
                .with_status("Location of Temporary Work Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "help_files",
                    "Help Files".to_string(),
                    ListValue::Path(25, icy_board.config.paths.help_path.clone()),
                )
                .with_status("Location of Help Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "login_sec",
                    "Login Security Files".to_string(),
                    ListValue::Path(25, icy_board.config.paths.security_file_path.clone()),
                )
                .with_status("Location of Login Security Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "login_sec",
                    "Command Display Files".to_string(),
                    ListValue::Path(25, icy_board.config.paths.command_display_path.clone()),
                )
                .with_status("Location of Command Display Files"),
            ),
        ];

        let configuration_files: Vec<ConfigEntry> = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "PWRD/Security File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.pwrd_sec_level_file.clone()),
                )
                .with_status("Name/Location of PWRD/Security File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "User Trashcan File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.bad_users.clone()),
                )
                .with_status("Name/Location of User Trashcan File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Password Trashcan File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.bad_passwords.clone()),
                )
                .with_status("Name/Location of Password Trashcan File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "EMail Trashcan File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.bad_email.clone()),
                )
                .with_status("Name/Location of EMail Trashcan File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "VIP Users File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.vip_users.clone()),
                )
                .with_status("Name/Location of VIP Users File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Protocol Data File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.protocol_data_file.clone()),
                )
                .with_status("Name/Location of Protocol Data File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Multi-Lang. Data File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.language_file.clone()),
                )
                .with_status("Name/Location of Multi-Lang. Data File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Default CMD.LST File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.language_file.clone()),
                )
                .with_status("Name/Location of CMD.LST File"),
            ),
        ];

        let display_files = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "WELCOME File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.welcome.clone()),
                )
                .with_status("Name/Location of WELCOME File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "NEWUSER File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.newuser.clone()),
                )
                .with_status("Name/Location of NEWUSER File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "CLOSED File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.closed.clone()),
                )
                .with_status("Name/Location of CLOSED File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "WARNING File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.expire_warning.clone()),
                )
                .with_status("Name/Location of WARNING File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "EXPIRED File".to_string(),
                    ListValue::Path(25, icy_board.config.paths.expired.clone()),
                )
                .with_status("Name/Location of EXPIRED File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Conference Join Menu".to_string(),
                    ListValue::Path(25, icy_board.config.paths.conf_join_menu.clone()),
                )
                .with_status("Name/Location of Conference Join Menu File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "NOANSI Warning".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
                )
                .with_status("Name/Location of NOANSI Warning File"),
            ),
        ];

        let new_user_files = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "New Reg Survey".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
                )
                .with_status("Name/Location of NEWASK Survey File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "New Reg Answers".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
                )
                .with_status("Name/Location of NEWASK Survey Answers"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Logon Survey".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
                )
                .with_status("Name/Location of Logon Survey File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Logon Answers".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
                )
                .with_status("Name/Location of Logon Survey Answers"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Logoff Survey".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
                )
                .with_status("Name/Location of Logoff Survey File"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "pwrd_sec",
                    "Logoff Answers".to_string(),
                    ListValue::Path(25, icy_board.config.paths.no_ansi.clone()),
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
        }
    }

    fn prev(&mut self) {
        if self.state.selected > 0 {
            self.state.selected -= 1;

            if let Some(y) = self.state.item_pos.get(&self.state.selected) {
                if *y < self.state.first_row {
                    self.state.first_row = *y;
                    if self.state.first_row == 1 {
                        self.state.first_row = 0;
                    }
                }
            }
        }
    }

    fn next(&mut self) {
        let count = self.config.count();
        if self.state.selected < count - 1 {
            self.state.selected += 1;
            if let Some(y) = self.state.item_pos.get(&self.state.selected) {
                if *y >= self.state.area_height {
                    self.state.first_row = *y - self.state.area_height + 1;
                }
            }
        }
    }
}

impl TabPage for PathTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(&Margin { horizontal: 2, vertical: 2 });

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(&Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame, &mut self.state);
    }

    fn request_edit_mode(&mut self, _t: &mut TerminalType, _fs: bool) -> ResultState {
        self.state.in_edit = true;
        self.config.request_edit_mode(&self.state)
    }

    fn set_cursor_position(&self, frame: &mut Frame) {
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if !self.state.in_edit {
            match key.code {
                KeyCode::Char('k') | KeyCode::Up => self.prev(),
                KeyCode::Char('j') | KeyCode::Down => self.next(),
                _ => {}
            }
            return ResultState {
                in_edit_mode: false,
                status_line: self.config.get_item(self.state.selected).unwrap().status.clone(),
            };
        } else {
            let res = self.config.handle_key_press(key, &self.state);
            self.state.in_edit = res.in_edit_mode;
            res
        }
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
