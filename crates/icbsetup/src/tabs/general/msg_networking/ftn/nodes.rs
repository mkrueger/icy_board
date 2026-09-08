use std::sync::{Arc, Mutex};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{IcyBoard, ftn::FtnLink};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use jamjam::util::echomail::EchomailAddress;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
};

/// The systems this board exchanges mail with, which `PCBoard` kept per node.
pub struct NodeConfiguration<'a> {
    insert_table: InsertTable<'a>,
    icy_board: Arc<Mutex<IcyBoard>>,
    detail: crate::editors::EditorDialog<(usize, Arc<Mutex<IcyBoard>>)>,
}

impl<'a> NodeConfiguration<'a> {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let content_length = icy_board.lock().unwrap().ftn.links.len();
        let board = icy_board.clone();
        let insert_table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(content_length),
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("fido_node_header_node")).with_width(24),
                Column::new(get_text("fido_node_header_host")).with_width(34),
                Column::new(get_text("fido_node_header_areas")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                let board = board.lock().unwrap();
                let Some(link) = board.ftn.links.get(*i) else {
                    return Line::from(String::new());
                };
                match j {
                    0 => Line::from(link.to_5d()),
                    1 => Line::from(format!("{}:{}", link.host, link.port)),
                    2 => Line::from(link.areas.len().to_string()),
                    _ => Line::from(String::new()),
                }
            }),
            content_length,
        };
        Self {
            insert_table,
            icy_board,
            detail: crate::editors::EditorDialog::default(),
        }
    }

    fn open_editor(&mut self, selected: usize) {
        let board = self.icy_board.lock().unwrap();
        let Some(link) = board.ftn.links.get(selected) else {
            return;
        };
        let width = 18;
        let entry = vec![
            ConfigEntry::Item(
                ListItem::new(get_text("fido_node_node"), ListValue::Text(24, TextFlags::None, link.address.to_string()))
                    .with_status(get_text("fido_node_node-status"))
                    .with_label_width(width)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        if let Some(address) = EchomailAddress::parse(&value) {
                            board.lock().unwrap().ftn.links[*i].address = address;
                        }
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_node_domain"), ListValue::Text(24, TextFlags::None, link.domain.clone()))
                    .with_status(get_text("fido_node_domain-status"))
                    .with_label_width(width)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().ftn.links[*i].domain = value;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_node_host"), ListValue::Text(48, TextFlags::None, link.host.clone()))
                    .with_status(get_text("fido_node_host-status"))
                    .with_label_width(width)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().ftn.links[*i].host = value;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_node_port"), ListValue::U32(u32::from(link.port), 0, u32::from(u16::MAX)))
                    .with_status(get_text("fido_node_port-status"))
                    .with_label_width(width)
                    .with_update_u32_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: u32| {
                        board.lock().unwrap().ftn.links[*i].port = value as u16;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_node_password"), ListValue::Text(24, TextFlags::Password, link.password.clone()))
                    .with_status(get_text("fido_node_password-status"))
                    .with_label_width(width)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().ftn.links[*i].password = value;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("fido_node_packet_password"),
                    ListValue::Text(8, TextFlags::Password, link.packet_password.clone()),
                )
                .with_status(get_text("fido_node_packet_password-status"))
                .with_help(get_text("fido_node_packet_password-help"))
                .with_label_width(width)
                .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                    board.lock().unwrap().ftn.links[*i].packet_password = value;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("fido_node_areafix_password"),
                    ListValue::Text(32, TextFlags::Password, link.area_fix_password.clone()),
                )
                .with_status(get_text("fido_node_areafix_password-status"))
                .with_help(get_text("fido_node_areafix_password-help"))
                .with_label_width(width)
                .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                    board.lock().unwrap().ftn.links[*i].area_fix_password = value;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_node_areas"), ListValue::Text(60, TextFlags::None, link.areas.join(" ")))
                    .with_status(get_text("fido_node_areas-status"))
                    .with_help(get_text("fido_node_areas-help"))
                    .with_label_width(width)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().ftn.links[*i].areas = value.split_whitespace().map(str::to_ascii_uppercase).collect();
                    }),
            ),
        ];
        drop(board);
        self.detail.open(ConfigMenu {
            obj: (selected, self.icy_board.clone()),
            entry,
        });
    }
}

impl<'a> Page for NodeConfiguration<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        crate::editors::list_editor_frame(get_text("fido_node_title"), "icb_setup_key_conf_list_help", self.detail.is_open()).render(area, frame.buffer_mut());

        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            self.detail.render(frame, area, get_text("fido_node_editor"), "");
        }
    }

    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::Insert => {
                self.insert_table.push_row(&mut self.icy_board.lock().unwrap().ftn.links, FtnLink::default());
            }
            KeyCode::Delete => {
                self.insert_table.remove_row(&mut self.icy_board.lock().unwrap().ftn.links);
            }
            KeyCode::Enter => {
                if let Some(selected) = self.insert_table.table_state.selected() {
                    self.open_editor(selected);
                }
            }
            _ => {
                let _ = self.insert_table.handle_key_press(key);
            }
        }
        PageMessage::None
    }
}
