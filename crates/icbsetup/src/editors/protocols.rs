use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        xfer_protocols::{Protocol, SupportedProtocols},
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use icy_net::protocol::TransferProtocolType;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
};

pub struct ProtocolEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    protocols_orig: SupportedProtocols,
    protocols: Arc<Mutex<SupportedProtocols>>,

    detail: super::EditorDialog<(usize, Arc<Mutex<SupportedProtocols>>)>,
    save_changes: super::EditorSaveChanges,
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
            columns: vec![
                Column::new(get_text("protocol_editor_header_char_code")).with_width(12),
                Column::new(get_text("protocol_editor_header_description")),
            ],
            // PCBSETUP keys protocols by their letter instead of numbering them.
            numbered: false,
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(cmd2.lock().unwrap()[*i].char_code.to_string()),
                    1 => Line::from(cmd2.lock().unwrap()[*i].description.to_string()),
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
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }
}

impl<'a> Page for ProtocolEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());

        let block = super::list_editor_frame(
            get_text("protocol_editor_title"),
            "icb_setup_key_conf_list_help",
            self.detail.is_open() || self.save_changes.is_open(),
        );
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let area = area.inner(Margin { vertical: 6, horizontal: 3 });
            self.detail.render(frame, area, get_text("protocol_editor_editor"), "");
        }
        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            crate::editors::save_file(&self.path, || self.protocols.lock().unwrap().save(&self.path))
        }) {
            return message;
        }
        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.protocols_orig != *self.protocols.lock().unwrap());
            }
            _ => match key.code {
                KeyCode::PageUp => self.insert_table.move_row(&mut self.protocols.lock().unwrap(), -1),
                KeyCode::PageDown => self.insert_table.move_row(&mut self.protocols.lock().unwrap(), 1),

                KeyCode::Insert => {
                    self.insert_table.push_row(
                        &mut *self.protocols.lock().unwrap(),
                        Protocol {
                            is_enabled: true,
                            is_batch: false,
                            is_bi_directional: false,
                            char_code: "N".to_string(),
                            description: String::new(),
                            send_command: TransferProtocolType::None,
                            recv_command: TransferProtocolType::None,
                        },
                    );
                }
                KeyCode::Delete => {
                    self.insert_table.remove_row(&mut *self.protocols.lock().unwrap());
                }

                KeyCode::Enter => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.protocols.lock().unwrap();
                        let Some(cur_prot) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.detail.open(ConfigMenu {
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
