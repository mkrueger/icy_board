use std::sync::{Arc, Mutex};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{IcyBoard, ftn::FtnRoute};
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

pub struct RoutingConfiguration<'a> {
    table: InsertTable<'a>,
    board: Arc<Mutex<IcyBoard>>,
    detail: crate::editors::EditorDialog<(usize, Arc<Mutex<IcyBoard>>)>,
}

impl<'a> RoutingConfiguration<'a> {
    pub fn new(board: Arc<Mutex<IcyBoard>>) -> Self {
        let content_length = board.lock().unwrap().ftn.routes.len();
        let content = board.clone();
        Self {
            table: InsertTable {
                scroll_state: ScrollbarState::default().content_length(content_length),
                table_state: TableState::default().with_selected(0),
                columns: vec![
                    Column::new(get_text("fido_route_header_destination")).with_width(28),
                    Column::new(get_text("fido_route_header_via")),
                ],
                numbered: true,
                get_content: Box::new(move |_table, i, j| {
                    let board = content.lock().unwrap();
                    let Some(route) = board.ftn.routes.get(*i) else {
                        return Line::from(String::new());
                    };
                    match j {
                        0 => Line::from(route.destination.to_string()),
                        1 => Line::from(route.via.to_string()),
                        _ => Line::from(String::new()),
                    }
                }),
                content_length,
            },
            board,
            detail: crate::editors::EditorDialog::default(),
        }
    }

    fn open_editor(&mut self, selected: usize) {
        let board = self.board.lock().unwrap();
        let Some(route) = board.ftn.routes.get(selected) else {
            return;
        };
        let entry = vec![
            ConfigEntry::Item(
                ListItem::new(
                    get_text("fido_route_destination"),
                    ListValue::Text(24, TextFlags::None, route.destination.to_string()),
                )
                .with_status(get_text("fido_route_destination-status"))
                .with_label_width(14)
                .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                    if let Some(address) = EchomailAddress::parse(&value) {
                        board.lock().unwrap().ftn.routes[*i].destination = address;
                    }
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_route_via"), ListValue::Text(24, TextFlags::None, route.via.to_string()))
                    .with_status(get_text("fido_route_via-status"))
                    .with_help(get_text("fido_route_via-help"))
                    .with_label_width(14)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        if let Some(address) = EchomailAddress::parse(&value) {
                            board.lock().unwrap().ftn.routes[*i].via = address;
                        }
                    }),
            ),
        ];
        drop(board);
        self.detail.open(ConfigMenu {
            obj: (selected, self.board.clone()),
            entry,
        });
    }
}

impl Page for RoutingConfiguration<'_> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        crate::editors::list_editor_frame(get_text("fido_route_title"), get_text("icb_setup_key_conf_list_help"), self.detail.is_open())
            .render(area, frame.buffer_mut());

        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.table.render_list(frame, area);

        if self.detail.is_open() {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            self.detail.render(frame, area, get_text("fido_route_editor"), String::new());
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
                self.table.push_row(&mut self.board.lock().unwrap().ftn.routes, FtnRoute::default());
            }
            KeyCode::Delete => {
                self.table.remove_row(&mut self.board.lock().unwrap().ftn.routes);
            }
            KeyCode::Enter => {
                if let Some(selected) = self.table.table_state.selected() {
                    self.open_editor(selected);
                }
            }
            _ => {
                let _ = self.table.handle_key_press(key);
            }
        }
        PageMessage::None
    }
}
