use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{IcyBoard, ftn::FtnRoute};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use jamjam::util::echomail::EchomailAddress;
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
};

pub struct RoutingConfiguration<'a> {
    table: InsertTable<'a>,
    board: Arc<Mutex<IcyBoard>>,
    edit_state: ConfigMenuState,
    editor: Option<ConfigMenu<(usize, Arc<Mutex<IcyBoard>>)>>,
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
            edit_state: ConfigMenuState::default(),
            editor: None,
        }
    }

    fn open_editor(&mut self, selected: usize) {
        self.edit_state = ConfigMenuState::default();
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
        self.editor = Some(ConfigMenu {
            obj: (selected, self.board.clone()),
            entry,
        });
    }
}

impl Page for RoutingConfiguration<'_> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        Block::new()
            .title_alignment(Alignment::Center)
            .title(Line::from(Span::from(get_text("fido_route_title")).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding))
            .render(area, frame.buffer_mut());

        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        let selected = self.table.table_state.selected();
        self.table.render_table(frame, area);
        self.table.table_state.select(selected);

        if let Some(editor) = &mut self.editor {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            Clear.render(area, frame.buffer_mut());
            Block::new()
                .title_alignment(Alignment::Center)
                .title(Line::from(Span::from(get_text("fido_route_editor")).style(get_tui_theme().dialog_box_title)))
                .style(get_tui_theme().dialog_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .render(area, frame.buffer_mut());
            editor.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_state);
            if let Some(item) = editor.get_item(self.edit_state.selected) {
                item.text_field_state.set_cursor_position(frame);
            }
        }
    }

    fn request_status(&self) -> ResultState {
        ResultState::default()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(editor) = &mut self.editor {
            if editor.handle_key_press(key, &mut self.edit_state).edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.editor = None;
            }
            return PageMessage::None;
        }
        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::Insert => {
                self.board.lock().unwrap().ftn.routes.push(FtnRoute::default());
                self.table.content_length += 1;
            }
            KeyCode::Delete => {
                if let Some(selected) = self.table.table_state.selected()
                    && selected < self.board.lock().unwrap().ftn.routes.len()
                {
                    self.board.lock().unwrap().ftn.routes.remove(selected);
                    self.table.content_length -= 1;
                }
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
