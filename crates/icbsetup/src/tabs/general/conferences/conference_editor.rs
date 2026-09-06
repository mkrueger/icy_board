use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{IcyBoard, security_expr::SecurityExpression, user_base::Password};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{Block, Borders, Padding, Widget},
};

pub struct ConferenceEditor {
    state: ConfigMenuState,
    menu: ConfigMenu<(usize, Arc<Mutex<IcyBoard>>)>,
}

static mut CUR_CONFERENCE: String = String::new();

// Header positions use the same cell offsets as the fields below them.
fn column_heading(columns: &[(usize, &str)]) -> String {
    let mut heading = String::new();
    for (offset, key) in columns {
        let width = ratatui::text::Line::raw(&heading).width();
        heading.push_str(&" ".repeat(offset.saturating_sub(width)));
        heading.push_str(&get_text(key));
    }
    heading
}

#[allow(static_mut_refs)]
pub fn get_cur_conference_name() -> String {
    unsafe { CUR_CONFERENCE.clone() }
}

impl ConferenceEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, num_conf: usize) -> Self {
        let conf_name = get_text_args("conf_name", HashMap::from([("number".to_string(), num_conf.to_string())]));

        let menu: ConfigMenu<(usize, Arc<Mutex<IcyBoard>>)> = {
            let ib = icy_board.lock().unwrap();
            let conf = ib.conferences.get(num_conf).unwrap();
            unsafe {
                CUR_CONFERENCE = if conf.name.is_empty() { conf_name.clone() } else { conf.name.clone() };
            }

            let name_block_width = 34;

            let table_width = 20;
            let lpath_width = 24;
            let rpath_width = 25;

            let opt_width = 29;
            let opt_width_right = 31;
            let opt_width_edit = 3;

            let upload_label_width = 15;
            let upload_sort_width = 5;
            let upload_metadata_width = 23;
            let upload_path_width = 25;
            let sort_offset = upload_label_width + 3;
            let metadata_offset = sort_offset + upload_sort_width + 1;
            let upload_offset = metadata_offset + upload_metadata_width + 1;

            let entry = vec![
                ConfigEntry::Item(
                    ListItem::new(conf_name, ListValue::Text(25, TextFlags::None, conf.name.clone()))
                        .with_label_width(name_block_width)
                        .with_update_text_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].name = value;
                        }),
                ),
                ConfigEntry::Table(
                    1,
                    vec![
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_public_conf"), ListValue::Bool(conf.is_public))
                                .with_label_width(name_block_width)
                                .with_edit_width(7)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].is_public = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("conf_req_sec_if_pub"),
                                ListValue::Security(conf.required_security.clone(), conf.required_security.to_string()),
                            )
                            .with_label_width(name_block_width)
                            .with_edit_width(14)
                            .with_update_sec_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: SecurityExpression| {
                                let mut ib = board.1.lock().unwrap();
                                ib.conferences[board.0].required_security = value;
                            }),
                        ),
                    ],
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("conf_pw_join_priv"),
                        ListValue::Text(12, TextFlags::Password, conf.password.to_string()),
                    )
                    .with_label_width(name_block_width)
                    .with_update_text_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        let mut ib = board.1.lock().unwrap();
                        ib.conferences[board.0].password = Password::PlainText(value);
                    }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_user_menu"), ListValue::Path(conf.users_menu.clone()))
                        .with_label_width(name_block_width)
                        .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].users_menu = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_sysop_menu"), ListValue::Path(conf.sysop_menu.clone()))
                        .with_label_width(name_block_width)
                        .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].sysop_menu = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_news_file"), ListValue::Path(conf.news_file.clone()))
                        .with_label_width(name_block_width)
                        .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].news_file = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_intro_file"), ListValue::Path(conf.intro_file.clone()))
                        .with_label_width(name_block_width)
                        .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].intro_file = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_attach_loc"), ListValue::Path(conf.attachment_location.clone()))
                        .with_label_width(name_block_width)
                        .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].attachment_location = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_cmd_lst_file"), ListValue::Path(conf.command_file.clone()))
                        .with_label_width(name_block_width)
                        .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].command_file = value;
                        }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(column_heading(&[
                    (sort_offset as usize, "conf_upload_sort_header"),
                    (metadata_offset as usize, "conf_upload_metadata_header"),
                    (upload_offset as usize, "conf_upload_location_header"),
                ])),
                ConfigEntry::Table(
                    3,
                    vec![
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_pub_upld"), ListValue::U32(conf.pub_upload_sort as u32, 0, 4))
                                .with_label_width(upload_label_width)
                                .with_edit_width(upload_sort_width)
                                .with_update_u32_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: u32| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].pub_upload_sort = value as u8;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.pub_upload_metadata.clone()))
                                .with_edit_width(upload_metadata_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].pub_upload_metadata = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.pub_upload_location.clone()))
                                .with_edit_width(upload_path_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].pub_upload_location = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_priv_upld"), ListValue::U32(conf.private_upload_sort as u32, 0, 4))
                                .with_label_width(upload_label_width)
                                .with_edit_width(upload_sort_width)
                                .with_update_u32_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: u32| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].private_upload_sort = value as u8;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.private_upload_metadata.clone()))
                                .with_edit_width(upload_metadata_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].private_upload_metadata = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.private_upload_location.clone()))
                                .with_edit_width(upload_path_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].private_upload_location = value;
                                }),
                        ),
                    ],
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(column_heading(&[
                    ((table_width + 3) as usize, "conf_menu_display_header"),
                    ((table_width + 3 + lpath_width + 1) as usize, "conf_menu_list_header"),
                ])),
                ConfigEntry::Table(
                    2,
                    vec![
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_doors"), ListValue::Path(conf.doors_menu.clone()))
                                .with_label_width(table_width)
                                .with_edit_width(lpath_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].doors_menu = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.doors_file.clone()))
                                .with_edit_width(rpath_width)
                                .with_path_editor(Box::new(crate::editors::door::edit_doors))
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].doors_file = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_bulletins"), ListValue::Path(conf.blt_menu.clone()))
                                .with_label_width(table_width)
                                .with_edit_width(lpath_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].blt_menu = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.blt_file.clone()))
                                .with_edit_width(rpath_width)
                                .with_path_editor(Box::new(crate::editors::bullettins::edit_bulletins))
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].blt_file = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_surveys"), ListValue::Path(conf.survey_menu.clone()))
                                .with_label_width(table_width)
                                .with_edit_width(lpath_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].survey_menu = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.survey_file.clone()))
                                .with_edit_width(rpath_width)
                                .with_path_editor(Box::new(crate::editors::surveys::edit_surveys))
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].survey_file = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_directories"), ListValue::Path(conf.dir_menu.clone()))
                                .with_label_width(table_width)
                                .with_edit_width(lpath_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].dir_menu = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.dir_file.clone()))
                                .with_edit_width(rpath_width)
                                .with_path_editor(Box::new(crate::editors::dirs::edit_dirs))
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].dir_file = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_areas"), ListValue::Path(conf.area_menu.clone()))
                                .with_label_width(table_width)
                                .with_edit_width(lpath_width)
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].area_menu = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new("".to_string(), ListValue::Path(conf.area_file.clone()))
                                .with_edit_width(rpath_width)
                                .with_path_editor(Box::new(crate::editors::areas::edit_areas))
                                .with_update_path_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].area_file = value;
                                }),
                        ),
                    ],
                ),
                ConfigEntry::Separator,
                ConfigEntry::Table(
                    2,
                    vec![
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_auto_rejon"), ListValue::Bool(conf.auto_rejoin))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].is_public = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_add_conf_sec"), ListValue::U32(conf.add_conference_security as u32, 0, 255))
                                .with_label_width(opt_width_right)
                                .with_edit_width(opt_width_edit)
                                .with_update_u32_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: u32| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].add_conference_security = value as i32;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_allow_view_conf_members"), ListValue::Bool(conf.allow_view_conf_members))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].allow_view_conf_members = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_add_conference_time"), ListValue::U32(conf.add_conference_time as u32, 0, 255))
                                .with_label_width(opt_width_right)
                                .with_edit_width(opt_width_edit)
                                .with_update_u32_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: u32| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].add_conference_time = value as u16;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_private_uploads"), ListValue::Bool(conf.private_uploads))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].private_uploads = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("conf_sec_attachments"),
                                ListValue::Security(conf.sec_attachments.clone(), conf.sec_attachments.to_string()),
                            )
                            .with_label_width(opt_width_right)
                            .with_edit_width(opt_width_edit)
                            .with_update_sec_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: SecurityExpression| {
                                let mut ib = board.1.lock().unwrap();
                                ib.conferences[board.0].sec_attachments = value;
                            }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_private_messages"), ListValue::Bool(conf.private_msgs))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].private_msgs = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("conf_sec_write_message"),
                                ListValue::Security(conf.sec_write_message.clone(), conf.sec_write_message.to_string()),
                            )
                            .with_label_width(opt_width_right)
                            .with_edit_width(opt_width_edit)
                            .with_update_sec_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: SecurityExpression| {
                                let mut ib = board.1.lock().unwrap();
                                ib.conferences[board.0].sec_write_message = value;
                            }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_echo_mail_in_conference"), ListValue::Bool(conf.echo_mail_in_conference))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].echo_mail_in_conference = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("conf_sec_carbon_copy"),
                                ListValue::Security(conf.sec_carbon_copy.clone(), conf.sec_carbon_copy.to_string()),
                            )
                            .with_label_width(opt_width_right)
                            .with_edit_width(opt_width_edit)
                            .with_update_sec_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: SecurityExpression| {
                                let mut ib = board.1.lock().unwrap();
                                ib.conferences[board.0].sec_carbon_copy = value;
                            }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_is_read_only"), ListValue::Bool(conf.is_read_only))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].is_read_only = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_carbon_list_limit"), ListValue::U32(conf.carbon_list_limit as u32, 0, 255))
                                .with_label_width(opt_width_right)
                                .with_edit_width(opt_width_edit)
                                .with_update_u32_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: u32| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].carbon_list_limit = value as u8;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_disallow_private_msgs"), ListValue::Bool(conf.disallow_private_msgs))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].disallow_private_msgs = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_charge_time"), ListValue::Float(conf.charge_time, conf.charge_time.to_string()))
                                .with_label_width(opt_width_right)
                                .with_edit_width(opt_width_edit)
                                .with_update_float_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: f64| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].charge_time = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(get_text("conf_allow_aliases"), ListValue::Bool(conf.allow_aliases))
                                .with_label_width(opt_width)
                                .with_edit_width(opt_width_edit)
                                .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                                    let mut ib = board.1.lock().unwrap();
                                    ib.conferences[board.0].allow_aliases = value;
                                }),
                        ),
                        ConfigEntry::Item(
                            ListItem::new(
                                get_text("conf_charge_msg_read"),
                                ListValue::Float(conf.charge_msg_read, conf.charge_msg_read.to_string()),
                            )
                            .with_label_width(opt_width_right)
                            .with_edit_width(opt_width_edit)
                            .with_update_float_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: f64| {
                                let mut ib = board.1.lock().unwrap();
                                ib.conferences[board.0].charge_msg_read = value;
                            }),
                        ),
                    ],
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("conf_show_intro_in_scan"), ListValue::Bool(conf.show_intro_in_scan))
                        .with_label_width(name_block_width)
                        .with_edit_width(opt_width_edit)
                        .with_update_bool_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: bool| {
                            let mut ib = board.1.lock().unwrap();
                            ib.conferences[board.0].show_intro_in_scan = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("conf_charge_msg_write"),
                        ListValue::Float(conf.charge_msg_write, conf.charge_msg_write.to_string()),
                    )
                    .with_label_width(name_block_width)
                    .with_edit_width(4)
                    .with_update_float_value(&|board: &(usize, Arc<Mutex<IcyBoard>>), value: f64| {
                        let mut ib = board.1.lock().unwrap();
                        ib.conferences[board.0].charge_msg_write = value;
                    }),
                ),
            ];
            ConfigMenu {
                obj: (num_conf, icy_board.clone()),
                entry,
            }
        };
        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for ConferenceEditor {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let area = Rect {
            x: disp_area.x + 1,
            y: disp_area.y,
            width: disp_area.width.saturating_sub(2),
            height: disp_area.height,
        };

        let mut bottom_text = get_text("icb_setup_key_menu_help");
        if let Some(item) = self.menu.get_item(self.state.selected)
            && let ListValue::Path(path) = &item.value
        {
            let path = self.menu.obj.1.lock().unwrap().resolve_file(path);
            if path.exists() && path.is_file() && item.editable() {
                bottom_text = get_text("icb_setup_key_menu_edit_help");
            }
        }

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
            width: disp_area.width.saturating_sub(5),
            height: area.height - 2,
        };
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: icy_board_tui::config_menu::EditMessage::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(item) = self.menu.get_item(self.state.selected)
            && let ListValue::Path(path) = &item.value
            && key.code == crossterm::event::KeyCode::F(2)
            && item.editable()
        {
            let path = self.menu.obj.1.lock().unwrap().resolve_file(path);
            if let Some(editor) = &item.path_editor {
                return editor(self.menu.obj.clone(), path);
            }

            let editor = &self.menu.obj.1.lock().unwrap().config.sysop.external_editor;
            let started = icy_board_tui::term::with_terminal(|| {
                std::process::Command::new(editor)
                    .arg(format!("{}", path.display()))
                    .spawn()
                    .and_then(|mut child| child.wait())
            });
            match started {
                Ok(_) => {
                    return PageMessage::ExternalProgramStarted;
                }
                Err(e) => {
                    log::error!("Error opening editor: {}", e);
                    return PageMessage::ResultState(ResultState {
                        edit_msg: icy_board_tui::config_menu::EditMessage::None,
                        status_line: format!("Error: {}", e),
                    });
                }
            }
        }

        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
            return PageMessage::Close;
        }
        PageMessage::ResultState(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    use icy_board_engine::icy_board::conferences::Conference;
    use ratatui::{Terminal, backend::TestBackend, text::Line};

    #[test]
    fn conference_labels_headers_and_editors_fit_when_scrolling() {
        let mut board = IcyBoard::default();
        board.conferences.clear();
        board.conferences.push(Conference {
            name: "Layout test".to_string(),
            pub_upload_metadata: "PUB-META/long/path/to/the/file-index".into(),
            private_upload_metadata: "PRIV-META/long/path/to/the/file-index".into(),
            pub_upload_location: "PUB-UPLOAD/long/path/to/the/upload-directory".into(),
            private_upload_location: "PRIV-UPLOAD/long/path/to/the/upload-directory".into(),
            doors_menu: "DOOR-MENU/long/path/to/the/menu".into(),
            doors_file: "DOOR-LIST/long/path/to/the/list.toml".into(),
            ..Default::default()
        });
        let mut page = ConferenceEditor::new(Arc::new(Mutex::new(board)), 0);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        let mut keys = vec![
            (1, "conf_public_conf"),
            (2, "conf_req_sec_if_pub"),
            (3, "conf_pw_join_priv"),
            (4, "conf_user_menu"),
            (5, "conf_sysop_menu"),
            (6, "conf_news_file"),
            (7, "conf_intro_file"),
            (8, "conf_attach_loc"),
            (9, "conf_cmd_lst_file"),
            (10, "conf_pub_upld"),
            (13, "conf_priv_upld"),
            (16, "conf_doors"),
            (18, "conf_bulletins"),
            (20, "conf_surveys"),
            (22, "conf_directories"),
            (24, "conf_areas"),
        ];
        keys.extend(
            [
                "conf_auto_rejon",
                "conf_add_conf_sec",
                "conf_allow_view_conf_members",
                "conf_add_conference_time",
                "conf_private_uploads",
                "conf_sec_attachments",
                "conf_private_messages",
                "conf_sec_write_message",
                "conf_echo_mail_in_conference",
                "conf_sec_carbon_copy",
                "conf_is_read_only",
                "conf_carbon_list_limit",
                "conf_disallow_private_msgs",
                "conf_charge_time",
                "conf_allow_aliases",
                "conf_charge_msg_read",
                "conf_show_intro_in_scan",
                "conf_charge_msg_write",
            ]
            .into_iter()
            .enumerate()
            .map(|(i, key)| (i + 26, key)),
        );
        let mut seen = std::collections::HashSet::new();
        for direction in [KeyCode::Down, KeyCode::Up] {
            for _ in 0..page.menu.count() {
                terminal.draw(|frame| page.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
                let buffer = terminal.backend().buffer();
                let row_text = |y| -> String { (0..80).map(|x| buffer[(x, y)].symbol()).collect() };
                for &(index, key) in &keys {
                    let row = page.state.item_pos[&index];
                    if row >= page.state.first_row && row < page.state.first_row + page.state.area_height {
                        let line = row_text(2 + row - page.state.first_row);
                        let label = get_text(key);
                        assert!(
                            line.split_once(&label).is_some_and(|(_, rest)| rest.trim_start().starts_with(':')),
                            "clipped {label}: {line}"
                        );
                        seen.insert(key);
                    }
                }
                for y in 2..23 {
                    assert_eq!(buffer[(78, y)].symbol(), "║", "overwritten right border at {y}");
                    assert_eq!(buffer[(79, y)].symbol(), " ", "text escaped the page at {y}");
                    assert_ne!(buffer[(79, y)].bg, get_tui_theme().edit_value.bg.unwrap(), "editor escaped the page at {y}");
                }
                // The four upper fields share one value column.
                if page.state.first_row == 0 {
                    for y in 2..6 {
                        assert_eq!(buffer[(38, y)].symbol(), ":");
                    }
                }
                for (header_key, marker) in [
                    ("conf_upload_sort_header", "0"),
                    ("conf_upload_metadata_header", "PUB-META"),
                    ("conf_upload_location_header", "PUB-UPLOAD"),
                    ("conf_menu_display_header", "DOOR-MENU"),
                    ("conf_menu_list_header", "DOOR-LIST"),
                ] {
                    let heading = get_text(header_key);
                    for y in 2..22 {
                        let header_row = row_text(y);
                        if let Some((prefix, _)) = header_row.split_once(&heading) {
                            let column = Line::raw(prefix).width() as u16;
                            let data: String = (column..column + marker.len() as u16).map(|x| buffer[(x, y + 1)].symbol()).collect();
                            assert_eq!(data, marker, "misaligned {heading}");
                        }
                    }
                }
                page.handle_key_press(KeyEvent::from(direction));
            }
        }
        assert_eq!(seen.len(), keys.len(), "not every label was visited");
    }
}
