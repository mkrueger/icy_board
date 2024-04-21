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

pub struct GeneralTab {
    pub state: ConfigMenuState,
    config: ConfigMenu,
}

impl GeneralTab {
    pub fn new(icy_board: Arc<IcyBoard>) -> Self {
        let sysop_info = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "sysop_name",
                    "Sysop's Name".to_string(),
                    ListValue::Text(25, icy_board.config.sysop.name.clone()),
                )
                .with_status("Enter the first name of the sysop."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "local_pass",
                    "Local Password".to_string(),
                    ListValue::Text(25, icy_board.config.sysop.password.to_string().clone()),
                )
                .with_status("Call waiting screen password."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "local_pass_exit",
                    "Require Password to Exit".to_string(),
                    ListValue::Bool(icy_board.config.sysop.require_password_to_exit),
                )
                .with_status("IcyBoard requires pw to exit the call waiting screen."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "use_real_name",
                    "Use Real Name".to_string(),
                    ListValue::Bool(icy_board.config.sysop.use_real_name),
                )
                .with_status("Message to sysop with real name?"),
            ),
        ];

        let board_info = vec![
            ConfigEntry::Item(
                ListItem::new("board_name", "Board Name".to_string(), ListValue::Text(54, icy_board.config.board.name.clone()))
                    .with_status("Board name is shown on login to the caller."),
            ),
            ConfigEntry::Item(
                ListItem::new("location", "Location".to_string(), ListValue::Text(54, icy_board.config.board.location.clone()))
                    .with_status("Board location used in IEMSI"),
            ),
            ConfigEntry::Item(
                ListItem::new("operator", "Operator".to_string(), ListValue::Text(30, icy_board.config.board.operator.clone()))
                    .with_status("Board operator used in IEMSI"),
            ),
            ConfigEntry::Item(
                ListItem::new("notice", "Notice".to_string(), ListValue::Text(30, icy_board.config.board.notice.clone()))
                    .with_status("Board notice used in IEMSI"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "capabilities",
                    "Capabilities".to_string(),
                    ListValue::Text(30, icy_board.config.board.capabilities.clone()),
                )
                .with_status("Board capabilities used in IEMSI"),
            ),
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    "date_format",
                    "Date Format".to_string(),
                    ListValue::Text(25, icy_board.config.board.date_format.clone()),
                )
                .with_status("Default date format"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "num_nodes",
                    "# Nodes".to_string(),
                    ListValue::Text(8, icy_board.config.board.num_nodes.to_string()),
                )
                .with_status("Numer of active nodes"),
            ),
        ];

        let new_user_info = vec![
            ConfigEntry::Item(ListItem::new(
                "sec_level",
                "Security Level".to_string(),
                ListValue::U32(icy_board.config.new_user_settings.sec_level as u32, 0, 255),
            )),
            ConfigEntry::Item(ListItem::new(
                "new_user_groups",
                "Groups".to_string(),
                ListValue::Text(40, icy_board.config.new_user_settings.new_user_groups.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_city_or_state",
                "City or State".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_city_or_state),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_address",
                "Address".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_address),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_verification",
                "Verification".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_verification),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_bus_data_phone",
                "Bus Phone".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_bus_data_phone),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_voice_phone",
                "Home Phone".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_voice_phone),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_comment",
                "Comment".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_comment),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_clr_msg",
                "MsgClear".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_clr_msg),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_xfer_protocol",
                "Protocols".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_xfer_protocol),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_date_format",
                "Date Format".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_date_format),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_alias",
                "Alias".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_alias),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_gender",
                "Gender".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_gender),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_birthdate",
                "Birthdate".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_birthdate),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_email",
                "Email".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_email),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_web_address",
                "Web Address".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_web_address),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_use_short_descr",
                "Short File Descr".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_use_short_descr),
            )),
        ];

        let mut function_keys = Vec::new();
        for i in 0..10 {
            let key = format!("f{}", i + 1);
            function_keys.push(ConfigEntry::Item(ListItem::new(
                &key,
                format!("F-Key #{}", i + 1),
                ListValue::Text(50, icy_board.config.func_keys[i].to_string()),
            )));
        }

        let message = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "max_lines",
                    "Max Message".to_string(),
                    ListValue::U32(icy_board.config.options.max_msg_lines as u32, 17, 400),
                )
                .with_status("Maximum Lines in the Message Editor (17-400)."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Scan ALL at Login".to_string(),
                    ListValue::Bool(icy_board.config.options.scan_all_mail_at_login),
                )
                .with_status("Default to Scan ALL Conferences at Login"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Read Mail Prompt".to_string(),
                    ListValue::Bool(icy_board.config.options.prompt_to_read_mail),
                )
                .with_status("Prompt to Read Mail w hen Mail Waiting"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Header DATE Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_date.clone()),
                )
                .with_status("Color for Message Header DATE Line"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Header TO Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_to.clone()),
                )
                .with_status("Color for Message Header TO Line"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Header FROM Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_from.clone()),
                )
                .with_status("Color for Message Header FROM Line"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Header SUBJ Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_subj.clone()),
                )
                .with_status("Color for Message Header SUBJ Line"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Header READ Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_read.clone()),
                )
                .with_status("Color for Message Header READ Line"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Header CONF Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_conf.clone()),
                )
                .with_status("Color for Message Header CONF Line"),
            ),
        ];

        let file_transfer = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Verify File Uploads".to_string(),
                    ListValue::Bool(icy_board.config.options.check_files_uploaded),
                )
                .with_status("Verify/Test uploaded files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Show 'Uploaded By'".to_string(),
                    ListValue::Bool(icy_board.config.options.display_uploader),
                )
                .with_status("Include 'Uploaded By' in Description"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Max UL descr Lines".to_string(),
                    ListValue::U32(icy_board.config.options.upload_descr_lines as u32, 1, 60),
                )
                .with_status("Maximum Number of Lines in Upload Description"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File HEAD Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_head.clone()),
                )
                .with_status("Color for File HEAD"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File NAME Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_name.clone()),
                )
                .with_status("Color for File NAME"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File Size Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_size.clone()),
                )
                .with_status("Color for Size of Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File DATE Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_date.clone()),
                )
                .with_status("Color for File DATE"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File DESCR1 Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_description.clone()),
                )
                .with_status("Color for File first line of Description"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File DESCR2 Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_description_low.clone()),
                )
                .with_status("Color for File Description"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File Text Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_text.clone()),
                )
                .with_status("Color for Text in Files"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "File Deleted Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_deleted.clone()),
                )
                .with_status("Color for 'Deleted' in Files"),
            ),
        ];
        let switches = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all",
                    "Keyboard Timeout".to_string(),
                    ListValue::U32(icy_board.config.options.keyboard_timeout as u32, 0, 255),
                )
                .with_status("Keyboard Timeout (in min, 0=off)"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "exclude_local_calls",
                    "Exclude Local".to_string(),
                    ListValue::Bool(icy_board.config.options.exclude_local_calls),
                )
                .with_status("Exclude Local Logins from Statistics"),
            ),
        ];

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                items: vec![
                    ConfigEntry::Group("Sysop Information".to_string(), sysop_info),
                    ConfigEntry::Group("Board Information".to_string(), board_info),
                    ConfigEntry::Group("New User Settings".to_string(), new_user_info),
                    ConfigEntry::Group("Function Keys".to_string(), function_keys),
                    ConfigEntry::Group("Messages".to_string(), message),
                    ConfigEntry::Group("File Transfer".to_string(), file_transfer),
                    ConfigEntry::Group("Switches".to_string(), switches),
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

impl TabPage for GeneralTab {
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
