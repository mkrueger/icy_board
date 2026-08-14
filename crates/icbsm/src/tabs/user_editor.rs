use chrono::{DateTime, Utc};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::PasswordStorageMethod,
    user_base::{ChatStatus, FSEMode, Password, User},
};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{Block, Borders, Padding, Widget},
};
use std::sync::{Arc, Mutex};

pub struct UserEditor {
    state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<User>>>,
    icy_board: Arc<Mutex<IcyBoard>>,
    num_user: usize,
    save_dialog: Option<SaveChangesDialog>,
}

impl UserEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, num_user: usize) -> Self {
        let user = icy_board.lock().unwrap().users.get(num_user).unwrap().clone();
        let password_storage_method = icy_board.lock().unwrap().config.system_control.password_storage_method;

        let menu: ConfigMenu<Arc<Mutex<User>>> = {
            let label_width = 14;
            let entry = vec![
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_name"), ListValue::Text(25, TextFlags::None, user.name.clone()))
                        .with_status(get_text("user_editor_name-status"))
                        .with_help(get_text("user_editor_name-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.name = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_alias"), ListValue::Text(25, TextFlags::None, user.alias.clone()))
                        .with_status(get_text("user_editor_alias-status"))
                        .with_help(get_text("user_editor_alias-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.alias = value;
                        }),
                ),
                ConfigEntry::Item({
                    let item = ListItem::new(get_text("user_editor_password"), ListValue::Text(25, TextFlags::Password, String::new()))
                        .with_status(get_text("user_editor_password-status"))
                        .with_help(get_text("user_editor_password-help"))
                        .with_label_width(label_width);
                    println!("Password storage method: {:?}", password_storage_method);
                    match password_storage_method {
                        PasswordStorageMethod::Argon2 => item.with_update_text_value(&|user: &Arc<Mutex<User>>, value: String| {
                            let mut user = user.lock().unwrap();
                            user.password.password = Password::new_argon2(value);
                        }),
                        PasswordStorageMethod::PlainText => item.with_update_text_value(&|user: &Arc<Mutex<User>>, value: String| {
                            let mut user = user.lock().unwrap();
                            user.password.password = Password::PlainText(value.to_lowercase());
                        }),
                        PasswordStorageMethod::BCrypt => item.with_update_text_value(&|user: &Arc<Mutex<User>>, value: String| {
                            let mut user = user.lock().unwrap();
                            user.password.password = Password::new_bcrypt(value);
                        }),
                    }
                }),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_security"), ListValue::U32(user.security_level as u32, 0, 255))
                        .with_status(get_text("user_editor_security-status"))
                        .with_help(get_text("user_editor_security-help"))
                        .with_label_width(label_width)
                        .with_update_u32_value(&|board: &Arc<Mutex<User>>, value: u32| {
                            let mut user = board.lock().unwrap();
                            user.security_level = value as u8;
                        }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_city"), ListValue::Text(25, TextFlags::None, user.city_or_state.clone()))
                        .with_status(get_text("user_editor_city-status"))
                        .with_help(get_text("user_editor_city-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.city_or_state = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_bus_phone"),
                        ListValue::Text(25, TextFlags::None, user.bus_data_phone.clone()),
                    )
                    .with_status(get_text("user_editor_bus_phone-status"))
                    .with_help(get_text("user_editor_bus_phone-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.bus_data_phone = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_home_phone"),
                        ListValue::Text(25, TextFlags::None, user.home_voice_phone.clone()),
                    )
                    .with_status(get_text("user_editor_home_phone-status"))
                    .with_help(get_text("user_editor_home_phone-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.home_voice_phone = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_verify_answer"),
                        ListValue::Text(25, TextFlags::None, user.verify_answer.clone()),
                    )
                    .with_status(get_text("user_editor_verify_answer-status"))
                    .with_help(get_text("user_editor_verify_answer-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.verify_answer = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_protocol"), ListValue::Text(5, TextFlags::None, user.protocol.clone()))
                        .with_status(get_text("user_editor_protocol-status"))
                        .with_help(get_text("user_editor_protocol-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.protocol = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_page_len"), ListValue::U32(user.page_len as u32, 0, 99))
                        .with_status(get_text("user_editor_page_len-status"))
                        .with_help(get_text("user_editor_page_len-help"))
                        .with_label_width(label_width)
                        .with_update_u32_value(&|board: &Arc<Mutex<User>>, value: u32| {
                            let mut user = board.lock().unwrap();
                            user.page_len = value as u16;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_reg_ex_date"), ListValue::date(user.expiration_date.clone()))
                        .with_status(get_text("user_editor_reg_ex_date-status"))
                        .with_help(get_text("user_editor_reg_ex_date-help"))
                        .with_label_width(label_width)
                        .with_update_date_value(&|board: &Arc<Mutex<User>>, value: DateTime<Utc>| {
                            let mut user = board.lock().unwrap();
                            user.expiration_date = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_exp_sec"), ListValue::U32(user.exp_security_level as u32, 0, 255))
                        .with_status(get_text("user_editor_exp_sec-status"))
                        .with_help(get_text("user_editor_exp_sec-help"))
                        .with_label_width(label_width)
                        .with_update_u32_value(&|board: &Arc<Mutex<User>>, value: u32| {
                            let mut user = board.lock().unwrap();
                            user.exp_security_level = value as u8;
                        }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Table(
                    2,
                    vec![
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_expert_mode"), ListValue::Bool(user.flags.expert_mode))
                                .with_status(get_text("user_editor_expert_mode-status"))
                                .with_help(get_text("user_editor_expert_mode-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.expert_mode = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_msg_clear"), ListValue::Bool(user.flags.msg_clear))
                                .with_status(get_text("user_editor_msg_clear-status"))
                                .with_help(get_text("user_editor_msg_clear-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.msg_clear = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_scroll_msg"), ListValue::Bool(user.flags.scroll_msg_body))
                                .with_status(get_text("user_editor_scroll_msg-status"))
                                .with_help(get_text("user_editor_scroll_msg-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.scroll_msg_body = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("user_editor_fse_mode"),
                                ListValue::Text(1, TextFlags::None, user.flags.fse_mode.to_char().to_string()),
                            )
                            .with_status(get_text("user_editor_fse_mode-status"))
                            .with_help(get_text("user_editor_fse_mode-help"))
                            .with_label_width(label_width)
                            .with_edit_width(1)
                            .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                                let mut user = board.lock().unwrap();
                                user.flags.fse_mode = FSEMode::from_pcboard(&value);
                            }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_use_short_filedescr"), ListValue::Bool(user.flags.use_short_filedescr))
                                .with_status(get_text("user_editor_use_short_filedescr-status"))
                                .with_help(get_text("user_editor_use_short_filedescr-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.use_short_filedescr = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_wide_editor"), ListValue::Bool(user.flags.wide_editor))
                                .with_status(get_text("user_editor_wide_editor-status"))
                                .with_help(get_text("user_editor_wide_editor-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.wide_editor = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("user_editor_last_conference"),
                                ListValue::U32(user.last_conference as u32, 0, u16::MAX as u32),
                            )
                            .with_status(get_text("user_editor_last_conference-status"))
                            .with_help(get_text("user_editor_last_conference-help"))
                            .with_label_width(label_width)
                            .with_update_u32_value(&|board: &Arc<Mutex<User>>, value: u32| {
                                let mut user = board.lock().unwrap();
                                user.last_conference = value as u16;
                            }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_long_msg_header"), ListValue::Bool(user.flags.long_msg_header))
                                .with_status(get_text("user_editor_long_msg_header-status"))
                                .with_help(get_text("user_editor_long_msg_header-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.long_msg_header = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_delete_user"), ListValue::Bool(user.flags.delete_flag))
                                .with_status(get_text("user_editor_delete_user-status"))
                                .with_help(get_text("user_editor_delete_user-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.flags.delete_flag = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("user_editor_chat_status"), ListValue::Bool(user.chat_status == ChatStatus::Available))
                                .with_status(get_text("user_editor_chat_status-status"))
                                .with_help(get_text("user_editor_chat_status-help"))
                                .with_label_width(label_width)
                                .with_update_bool_value(&|board: &Arc<Mutex<User>>, value: bool| {
                                    let mut user = board.lock().unwrap();
                                    user.chat_status = if value { ChatStatus::Available } else { ChatStatus::Unavailable };
                                }),
                        ),
                    ],
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_comment1"),
                        ListValue::Text(60, TextFlags::None, user.user_comment.clone()),
                    )
                    .with_status(get_text("user_editor_comment1-status"))
                    .with_help(get_text("user_editor_comment1-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.user_comment = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_comment2"),
                        ListValue::Text(60, TextFlags::None, user.sysop_comment.clone()),
                    )
                    .with_status(get_text("user_editor_comment2-status"))
                    .with_help(get_text("user_editor_comment2-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.sysop_comment = value;
                    }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_adr1"), ListValue::Text(25, TextFlags::None, user.street1.clone()))
                        .with_status(get_text("user_editor_adr1-status"))
                        .with_help(get_text("user_editor_adr1-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.street1 = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_adr2"), ListValue::Text(25, TextFlags::None, user.street2.clone()))
                        .with_status(get_text("user_editor_adr2-status"))
                        .with_help(get_text("user_editor_adr2-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.street2 = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_city"), ListValue::Text(25, TextFlags::None, user.city.clone()))
                        .with_status(get_text("user_editor_city-status"))
                        .with_help(get_text("user_editor_city-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.city = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_state"), ListValue::Text(25, TextFlags::None, user.state.clone()))
                        .with_status(get_text("user_editor_state-status"))
                        .with_help(get_text("user_editor_state-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.state = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_zip"), ListValue::Text(25, TextFlags::None, user.zip.clone()))
                        .with_status(get_text("user_editor_zip-status"))
                        .with_help(get_text("user_editor_zip-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.zip = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_country"), ListValue::Text(25, TextFlags::None, user.country.clone()))
                        .with_status(get_text("user_editor_country-status"))
                        .with_help(get_text("user_editor_country-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.country = value;
                        }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_cmt_line1"),
                        ListValue::Text(25, TextFlags::None, user.custom_comment1.clone()),
                    )
                    .with_status(get_text("user_editor_cmt_line1-status"))
                    .with_help(get_text("user_editor_cmt_line1-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.custom_comment1 = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_cmt_line2"),
                        ListValue::Text(25, TextFlags::None, user.custom_comment2.clone()),
                    )
                    .with_status(get_text("user_editor_cmt_line2-status"))
                    .with_help(get_text("user_editor_cmt_line2-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.custom_comment2 = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_cmt_line3"),
                        ListValue::Text(25, TextFlags::None, user.custom_comment3.clone()),
                    )
                    .with_status(get_text("user_editor_cmt_line3-status"))
                    .with_help(get_text("user_editor_cmt_line3-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.custom_comment3 = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_cmt_line4"),
                        ListValue::Text(25, TextFlags::None, user.custom_comment4.clone()),
                    )
                    .with_status(get_text("user_editor_cmt_line4-status"))
                    .with_help(get_text("user_editor_cmt_line4-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.custom_comment4 = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("user_editor_cmt_line5"),
                        ListValue::Text(25, TextFlags::None, user.custom_comment5.clone()),
                    )
                    .with_status(get_text("user_editor_cmt_line5-status"))
                    .with_help(get_text("user_editor_cmt_line5-help"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                        let mut user = board.lock().unwrap();
                        user.custom_comment5 = value;
                    }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_gender"), ListValue::Text(25, TextFlags::None, user.gender.clone()))
                        .with_status(get_text("user_editor_gender-status"))
                        .with_help(get_text("user_editor_gender-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.gender = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_birthdate"), ListValue::date(user.birth_date.clone()))
                        .with_label_width(label_width)
                        .with_status(get_text("user_editor_birthdate-status"))
                        .with_help(get_text("user_editor_birthdate-help"))
                        .with_update_date_value(&|board: &Arc<Mutex<User>>, value: DateTime<Utc>| {
                            let mut user = board.lock().unwrap();
                            user.expiration_date = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_email"), ListValue::Text(60, TextFlags::None, user.email.clone()))
                        .with_status(get_text("user_editor_email-status"))
                        .with_help(get_text("user_editor_email-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.email = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("user_editor_web"), ListValue::Text(60, TextFlags::None, user.web.clone()))
                        .with_status(get_text("user_editor_web-status"))
                        .with_help(get_text("user_editor_web-help"))
                        .with_label_width(label_width)
                        .with_update_text_value(&|board: &Arc<Mutex<User>>, value: String| {
                            let mut user = board.lock().unwrap();
                            user.web = value;
                        }),
                ),
            ];
            ConfigMenu {
                obj: Arc::new(Mutex::new(user)),
                entry,
            }
        };
        Self {
            state: ConfigMenuState::default(),
            menu,
            icy_board,
            num_user,
            save_dialog: None,
        }
    }
}

impl Page for UserEditor {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let area = Rect {
            x: disp_area.x + 1,
            y: disp_area.y,
            width: disp_area.width.saturating_sub(2),
            height: disp_area.height,
        };

        let bottom_text = get_text("icb_setup_key_menu_help");

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(Span::styled(bottom_text, get_tui_theme().key_binding))
            .border_style(get_tui_theme().dialog_box);
        block.render(area, frame.buffer_mut());

        let area = Rect {
            x: disp_area.x + 3,
            y: area.y + 1,
            width: disp_area.width - 3,
            height: area.height - 2,
        };
        self.menu.render(area, frame, &mut self.state);
        if let Some(save_changes) = &self.save_dialog {
            save_changes.render(frame, area);
        }
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: icy_board_tui::config_menu::EditMessage::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.save_dialog.is_some() {
            let res = self.save_dialog.as_mut().unwrap().handle_key_press(key);
            return match res {
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Cancel => {
                    self.save_dialog = None;
                    PageMessage::None
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Close => PageMessage::Close,
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Save => {
                    self.icy_board.lock().unwrap().users[self.num_user] = self.menu.obj.lock().unwrap().clone();
                    self.icy_board.lock().unwrap().save_userbase().unwrap();
                    PageMessage::Close
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
            if *self.menu.obj.lock().unwrap() == self.icy_board.lock().unwrap().users[self.num_user] {
                return PageMessage::Close;
            }
            self.save_dialog = Some(SaveChangesDialog::new());
            return PageMessage::None;
        }
        PageMessage::ResultState(res)
    }
}
