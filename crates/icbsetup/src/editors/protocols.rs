use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        xfer_protocols::{Protocol, SupportedProtocols},
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    get_text,
    insert_table::InsertTable,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use icy_net::protocol::TransferProtocolType;
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget, block::Title},
};

pub struct ProtocolEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    protocols_orig: SupportedProtocols,
    protocols: Arc<Mutex<SupportedProtocols>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<SupportedProtocols>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> ProtocolEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let protocols_orig = if path.exists() {
            SupportedProtocols::load(&path)?
        } else {
            SupportedProtocols::default()
        };

        let protocols = Arc::new(Mutex::new(protocols_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(protocols_orig.len());
        let content_length = protocols_orig.len();
        let cmd2 = protocols.clone();

        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![get_text("protocol_editor_header_char_code"), get_text("protocol_editor_header_description")],
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].char_code)),
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].description)),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };
        Ok(Self {
            path: path.clone(),
            protocols_orig,
            insert_table,
            protocols,
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
                let mut levels = self.protocols.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.protocols.lock().unwrap().len() {
                let mut levels = self.protocols.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Page for ProtocolEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(
                Span::from(get_text("protocol_editor_title")).style(get_tui_theme().dialog_box_title),
            ))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 6, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("protocol_editor_editor")).style(get_tui_theme().dialog_box_title),
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
                    self.protocols.lock().unwrap().save(&self.path).unwrap();
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
                if self.protocols_orig == self.protocols.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

                KeyCode::Insert => {
                    self.protocols.lock().unwrap().push(Protocol {
                        is_enabled: true,
                        is_batch: false,
                        is_bi_directional: false,
                        char_code: "N".to_string(),
                        description: String::new(),
                        send_command: TransferProtocolType::None,
                        recv_command: TransferProtocolType::None,
                    });
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.protocols.lock().unwrap().len() {
                            self.protocols.lock().unwrap().remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.protocols.lock().unwrap();
                        let Some(cur_prot) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.protocols.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("protocol_editor_char_code"),
                                        ListValue::Text(1, TextFlags::None, cur_prot.char_code.clone()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: String| {
                                            list.lock().unwrap()[*i].char_code = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("protocol_editor_description"),
                                        ListValue::Text(30, TextFlags::None, cur_prot.description.clone()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: String| {
                                            list.lock().unwrap()[*i].description = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("protocol_editor_is_enabled"), ListValue::Bool(cur_prot.is_enabled))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: bool| {
                                            list.lock().unwrap()[*i].is_enabled = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("protocol_editor_is_batch"), ListValue::Bool(cur_prot.is_batch))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: bool| {
                                            list.lock().unwrap()[*i].is_batch = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("protocol_editor_bidirectional"), ListValue::Bool(cur_prot.is_bi_directional))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: bool| {
                                            list.lock().unwrap()[*i].is_bi_directional = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("protocol_editor_send_cmd"),
                                        ListValue::Text(40, TextFlags::None, cur_prot.send_command.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: String| {
                                            list.lock().unwrap()[*i].send_command = TransferProtocolType::from(value);
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("protocol_editor_recv_cmd"),
                                        ListValue::Text(40, TextFlags::None, cur_prot.recv_command.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<SupportedProtocols>>), value: String| {
                                            list.lock().unwrap()[*i].recv_command = TransferProtocolType::from(value);
                                        },
                                    ),
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

pub fn edit_protocols(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(ProtocolEditor::new(&path).unwrap()))
}
