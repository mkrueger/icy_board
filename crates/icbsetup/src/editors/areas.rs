use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        message_area::{AreaList, MessageArea},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    get_text, get_text_args,
    insert_table::InsertTable,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget, block::Title},
};

pub struct MessageAreasEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    area_list_orig: AreaList,
    area_list: Arc<Mutex<AreaList>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<AreaList>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> MessageAreasEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let area_list_orig = if path.exists() { AreaList::load(&path)? } else { AreaList::default() };
        let area_list = Arc::new(Mutex::new(area_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(area_list_orig.len());
        let content_length = area_list_orig.len();
        let dl2 = area_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![
                "".to_string(),
                format!("{:<20}", get_text("dirs_table_name_header")),
                get_text("dirs_table_path_header"),
            ],
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", *i + 1)),
                    1 => Line::from(format!("{}", dl2.lock().unwrap()[*i].name)),
                    2 => Line::from(format!("{}", dl2.lock().unwrap()[*i].path.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            insert_table,
            area_list,
            area_list_orig,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            save_dialog: None,
        })
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect) {
        let sel = self.insert_table.table_state.selected();
        self.insert_table.render_table(frame, *area);
        self.insert_table.table_state.select(sel);
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected > 0 {
                let mut levels = self.area_list.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.area_list.lock().unwrap().len() {
                let mut levels = self.area_list.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Page for MessageAreasEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("area_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(title).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 3, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("area_editor_edit_title")).style(get_tui_theme().dialog_box_title),
                ))
                .style(get_tui_theme().dialog_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
            block.render(area, frame.buffer_mut());
            edit_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);

            edit_config
                .get_item(self.edit_config_state.selected)
                .unwrap()
                .text_field_state
                .set_cursor_position(frame);
        }
        if let Some(save_changes) = &self.save_dialog {
            save_changes.render(frame, area);
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
                    if let Some(parent) = self.path.parent() {
                        if !parent.exists() {
                            std::fs::create_dir_all(parent).unwrap();
                        }
                    }
                    self.area_list.lock().unwrap().save(&self.path).unwrap();
                    PageMessage::Close
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        if let Some(edit_config) = &mut self.edit_config {
            let res = edit_config.handle_key_press(key, &mut self.edit_config_state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.edit_config = None;
                return PageMessage::None;
            }
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => {
                if self.area_list_orig == self.area_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

                KeyCode::Insert => {
                    self.area_list.lock().unwrap().push(MessageArea::default());
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.area_list.lock().unwrap().len() {
                            self.area_list.lock().unwrap().remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.area_list.lock().unwrap();
                        let Some(item) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.area_list.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_name"), ListValue::Text(25, item.name.to_string()))
                                        .with_label_width(16)
                                        .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                            list.lock().unwrap()[*i].name = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_qwk_name"), ListValue::Text(25, item.qwk_name.to_string()))
                                        .with_label_width(16)
                                        .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                            list.lock().unwrap()[*i].qwk_name = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_file"), ListValue::Path(item.path.clone()))
                                        .with_label_width(16)
                                        .with_update_path_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: PathBuf| {
                                            list.lock().unwrap()[*i].path = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_is_readonly"), ListValue::Bool(item.is_read_only))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: bool| {
                                            list.lock().unwrap()[*i].is_read_only = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_allow_aliases"), ListValue::Bool(item.allow_aliases))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: bool| {
                                            list.lock().unwrap()[*i].allow_aliases = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_list_sec"),
                                        ListValue::Security(item.req_level_to_list.clone(), item.req_level_to_list.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<AreaList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].req_level_to_list = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_enter_sec"),
                                        ListValue::Security(item.req_level_to_enter.clone(), item.req_level_to_enter.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<AreaList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].req_level_to_enter = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_attach_sec"),
                                        ListValue::Security(item.req_level_to_save_attach.clone(), item.req_level_to_save_attach.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<AreaList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].req_level_to_save_attach = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_qwk_number"),
                                        ListValue::U32(item.qwk_conference_number as u32, 0, u16::MAX as u32),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                        list.lock().unwrap()[*i].qwk_name = value;
                                    }),
                                ),
                            ],
                        });
                    } else {
                        self.insert_table.handle_key_press(key).unwrap();
                    }
                }
                _ => {
                    self.insert_table.handle_key_press(key).unwrap();
                }
            },
        }
        PageMessage::None
    }
}

pub fn edit_areas(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(MessageAreasEditor::new(&path).unwrap()))
}
