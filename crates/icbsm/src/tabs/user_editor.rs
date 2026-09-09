use chrono::{DateTime, Utc};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::PasswordStorageMethod,
    password_recovery::security_fingerprint,
    user_base::{ChatStatus, FSEMode, Password, User},
    user_store::UserUpdateMode,
};
use icy_board_tui::{
    chrome::{dim_background, dirty_title},
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    hotkeys::HotkeyBar,
    icbconfigmenu::render_config_menu_frame,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{InfoState, Page, PageMessage},
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct UserEditor {
    state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<User>>>,
    icy_board: Arc<Mutex<IcyBoard>>,
    num_user: usize,
    baseline: User,
    new_user: Option<Arc<Mutex<User>>>,
    draft_changed: Option<Arc<AtomicBool>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl UserEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, num_user: usize) -> Self {
        let user = icy_board.lock().unwrap().users.get(num_user).unwrap().clone();
        Self::from_snapshot(icy_board, num_user, user)
    }

    pub(super) fn for_new_user(icy_board: Arc<Mutex<IcyBoard>>, num_user: usize, user: Arc<Mutex<User>>, draft_changed: Arc<AtomicBool>) -> Self {
        let snapshot = user.lock().unwrap().clone();
        let mut editor = Self::from_snapshot(icy_board, num_user, snapshot);
        editor.new_user = Some(user);
        editor.draft_changed = Some(draft_changed);
        editor
    }

    pub(super) fn from_snapshot(icy_board: Arc<Mutex<IcyBoard>>, num_user: usize, user: User) -> Self {
        let baseline = user.clone();
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
                    ListItem::new(get_text("user_editor_reg_ex_date"), ListValue::date(user.expiration_date))
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
                    ListItem::new(get_text("user_editor_birthdate"), ListValue::date(user.birth_date))
                        .with_label_width(label_width)
                        .with_status(get_text("user_editor_birthdate-status"))
                        .with_help(get_text("user_editor_birthdate-help"))
                        .with_update_date_value(&|board: &Arc<Mutex<User>>, value: DateTime<Utc>| {
                            let mut user = board.lock().unwrap();
                            user.birth_date = value;
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
            .with_fitted_labels()
        };
        Self {
            baseline,
            new_user: None,
            draft_changed: None,
            state: ConfigMenuState::default(),
            menu,
            icy_board,
            num_user,
            save_dialog: None,
        }
    }

    fn is_dirty(&self) -> bool {
        !same_user_edit(&self.menu.obj.lock().unwrap(), &self.baseline)
    }

    fn save(&mut self) -> icy_board_engine::Res<()> {
        let edited = self.menu.obj.lock().unwrap().clone();
        let saved = if let Some(user) = &self.new_user {
            // New list records stay private until the list transaction is saved.
            *user.lock().unwrap() = edited.clone();
            edited
        } else {
            self.icy_board.lock().unwrap().update_user(&self.baseline, &edited, UserUpdateMode::Edit)?
        };
        self.baseline = saved.clone();
        *self.menu.obj.lock().unwrap() = saved;
        if let Some(changed) = &self.draft_changed {
            changed.store(true, Ordering::Release);
        }
        Ok(())
    }
}

pub(super) fn same_user_edit(left: &User, right: &User) -> bool {
    macro_rules! same_fields {
        ($($field:ident),+ $(,)?) => { true $(&& left.$field == right.$field)+ };
    }
    // The fingerprint compares password records and history without verifying secrets.
    // Recovery challenges are authoritative, not editable form fields.
    security_fingerprint(left) == security_fingerprint(right)
        && same_fields!(
            path,
            verify_answer,
            city_or_state,
            city,
            state,
            street1,
            street2,
            zip,
            country,
            gender,
            web,
            contacts,
            date_format,
            language,
            bus_data_phone,
            home_voice_phone,
            birth_date,
            user_comment,
            sysop_comment,
            custom_comment1,
            custom_comment2,
            custom_comment3,
            custom_comment4,
            custom_comment5,
            credential_revision,
            security_stamp,
            recovery_issues,
            expiration_date,
            flags,
            protocol,
            page_len,
            last_conference,
            elapsed_time_on,
            date_last_dir_read,
            qwk_config,
            account,
            bank,
            stats,
            chat_status,
            conference_flags,
            lastread_ptr_flags,
            tpa_records,
        )
}

impl Page for UserEditor {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let dirty = self.is_dirty();
        let title = dirty_title(format!("{} #{}", get_text("icbsm_menu_edit_users"), self.num_user + 1), dirty);
        let footer = self.save_dialog.is_none().then(|| HotkeyBar::for_id("icb_setup_key_menu_help").line());
        let area = render_config_menu_frame(frame, disp_area, &title, footer);
        self.menu.render(area, frame, &mut self.state);
        if let Some(save_changes) = &self.save_dialog {
            let backdrop = frame.area();
            dim_background(frame.buffer_mut(), backdrop);
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
                    self.save_dialog = None;
                    match self.save() {
                        Ok(()) => PageMessage::Close,
                        Err(err) => PageMessage::InfoBox(
                            InfoState::Error,
                            get_text_args("icbsm_save_failed", std::collections::HashMap::from([("error".to_string(), err.to_string())])),
                        ),
                    }
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
            if !self.is_dirty() {
                return PageMessage::Close;
            }
            self.save_dialog = Some(SaveChangesDialog::new());
            return PageMessage::None;
        }
        PageMessage::ResultState(res)
    }
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    use crate::tabs::user_save_tests::Fixture;
    use crossterm::event::KeyCode;
    use icy_board_tui::hotkeys::HotkeyBar;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    fn confirm_save(editor: &mut UserEditor) -> PageMessage {
        assert!(matches!(editor.handle_key_press(KeyCode::Esc.into()), PageMessage::None));
        assert!(editor.save_dialog.is_some());
        editor.handle_key_press(KeyCode::Left.into());
        editor.handle_key_press(KeyCode::Enter.into())
    }

    #[test]
    fn editor_save_merges_other_field_updates_after_reordering() {
        let fixture = Fixture::new();
        let mut editor = UserEditor::new(fixture.board.clone(), 2);
        editor.menu.obj.lock().unwrap().sysop_comment = "editor draft".into();
        fixture
            .board
            .lock()
            .unwrap()
            .edit_users(|users| {
                users[2].city_or_state = "Hamburg".into();
                users[2].stats.num_times_on = 42;
                users.swap(1, 2);
                Ok(())
            })
            .unwrap();

        assert!(matches!(confirm_save(&mut editor), PageMessage::Close));
        assert!(!editor.is_dirty());
        let users = fixture.disk_users();
        assert_eq!(users[1].name, "Bob");
        assert_eq!(users[1].city_or_state, "Hamburg");
        assert_eq!(users[1].stats.num_times_on, 42);
        assert_eq!(users[1].sysop_comment, "editor draft");
        assert!(users[2].sysop_comment.is_empty());
        assert_eq!(editor.baseline.city_or_state, "Hamburg");
    }

    #[test]
    fn conflicting_editor_save_keeps_live_record_and_draft() {
        let mut fixture = Fixture::new();
        let mut editor = UserEditor::new(fixture.board.clone(), 1);
        editor.menu.obj.lock().unwrap().city_or_state = "Munich".into();
        fixture
            .board
            .lock()
            .unwrap()
            .edit_users(|users| {
                users[1].city_or_state = "Hamburg".into();
                Ok(())
            })
            .unwrap();
        fixture.persisted = std::fs::read(fixture.dir.join("users.toml")).unwrap();
        let before = fixture.snapshot();

        assert!(matches!(confirm_save(&mut editor), PageMessage::InfoBox(InfoState::Error, _)));
        fixture.assert_unchanged(&before);
        assert!(editor.is_dirty());
        assert_eq!(editor.menu.obj.lock().unwrap().city_or_state, "Munich");
        assert_eq!(editor.baseline.city_or_state, "Berlin");
    }

    #[test]
    fn editor_save_failure_does_not_normalize_or_publish_any_live_user() {
        let fixture = Fixture::new();
        let mut editor = UserEditor::new(fixture.board.clone(), 1);
        editor.menu.obj.lock().unwrap().sysop_comment = "draft".into();
        {
            let mut board = fixture.board.lock().unwrap();
            board.users[0].email = "pending-normalization@example.invalid".into();
            board.config.paths.user_file = fixture.dir.clone();
        }
        let before = fixture.snapshot();
        assert!(matches!(confirm_save(&mut editor), PageMessage::InfoBox(InfoState::Error, _)));
        fixture.assert_unchanged(&before);
        assert!(editor.is_dirty());
    }

    #[test]
    fn removed_editor_identity_never_targets_a_reused_index_or_alias() {
        let mut fixture = Fixture::new();
        let mut editor = UserEditor::new(fixture.board.clone(), 2);
        editor.menu.obj.lock().unwrap().sysop_comment = "draft".into();
        fixture
            .board
            .lock()
            .unwrap()
            .edit_users(|users| {
                users.remove(2);
                users[1].alias = "Bob".into();
                Ok(())
            })
            .unwrap();
        fixture.persisted = std::fs::read(fixture.dir.join("users.toml")).unwrap();
        let before = fixture.snapshot();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        assert!(matches!(confirm_save(&mut editor), PageMessage::InfoBox(InfoState::Error, _)));
        fixture.assert_unchanged(&before);
    }

    #[test]
    fn unchanged_argon2_snapshot_is_clean_even_when_live_identity_disappears() {
        let fixture = Fixture::new();
        {
            let mut board = fixture.board.lock().unwrap();
            board.users[2].password.password = Password::new_argon2("secret");
            board.users[2].password.prev_pwd.push(Password::new_argon2("previous"));
        }
        let mut editor = UserEditor::new(fixture.board.clone(), 2);
        assert!(!editor.is_dirty());
        editor.menu.obj.lock().unwrap().password.password = Password::PlainText("secret".into());
        assert!(editor.is_dirty(), "a storage change must not be treated as password verification");
        *editor.menu.obj.lock().unwrap() = editor.baseline.clone();
        fixture.board.lock().unwrap().users.remove(2);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        assert!(!row_text(terminal.backend().buffer(), 2).contains('*'));
        assert!(matches!(editor.handle_key_press(KeyCode::Esc.into()), PageMessage::Close));
    }

    #[test]
    fn new_user_editor_saves_only_to_the_pending_list_record() {
        let fixture = Fixture::new();
        let before = fixture.snapshot();
        let pending = Arc::new(Mutex::new(User {
            name: "NewUser4".into(),
            ..Default::default()
        }));
        let changed = Arc::new(AtomicBool::new(false));
        let mut editor = UserEditor::for_new_user(fixture.board.clone(), 3, pending.clone(), changed.clone());
        editor.menu.obj.lock().unwrap().name = "New account".into();
        assert!(!changed.load(Ordering::Acquire));
        assert!(matches!(confirm_save(&mut editor), PageMessage::Close));
        assert!(changed.load(Ordering::Acquire));
        assert_eq!(pending.lock().unwrap().name, "New account");
        assert!(!editor.is_dirty());
        fixture.assert_unchanged(&before);
    }

    #[test]
    fn dirty_title_and_save_backdrop_leave_the_user_form_geometry_unchanged() {
        let mut board = IcyBoard::default();
        board.users.new_user(User {
            name: "Alice".into(),
            ..Default::default()
        });
        let board = Arc::new(Mutex::new(board));
        let mut editor = UserEditor::new(board.clone(), 0);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        let clean_title = format!("{} #1", get_text("icbsm_menu_edit_users"));
        assert!(!row_text(terminal.backend().buffer(), 1).contains(&clean_title));
        assert!(row_text(terminal.backend().buffer(), 2).contains(&clean_title));
        assert_eq!(row_text(terminal.backend().buffer(), 3).chars().filter(|ch| *ch == '─').count(), 76);
        assert!(!row_text(terminal.backend().buffer(), 2).contains('*'));
        let visible = (0..25).map(|y| row_text(terminal.backend().buffer(), y)).collect::<Vec<_>>().join("\n");
        for label in [
            "user_editor_security",
            "user_editor_bus_phone",
            "user_editor_home_phone",
            "user_editor_use_short_filedescr",
            "user_editor_wide_editor",
        ] {
            let label = get_text(label);
            assert!(visible.contains(&label), "clipped label: {label}");
        }
        assert!(row_text(terminal.backend().buffer(), 24).contains(&HotkeyBar::for_id("icb_setup_key_menu_help").line().to_string()));

        editor.menu.obj.lock().unwrap().sysop_comment = "Unsaved draft".into();
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        assert!(row_text(terminal.backend().buffer(), 2).contains(&format!("{clean_title} *")));
        let mut expected = terminal.backend().buffer().clone();
        let area = expected.area;
        dim_background(&mut expected, area);

        editor.save_dialog = Some(SaveChangesDialog::new());
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        let actual = terminal.backend().buffer();
        for y in 0..5 {
            for x in 0..80 {
                assert_eq!(actual[(x, y)], expected[(x, y)]);
            }
        }
        assert!(!(20..25).any(|y| row_text(actual, y).contains("F1")));
        assert!(board.lock().unwrap().users[0].sysop_comment.is_empty());
    }
}
