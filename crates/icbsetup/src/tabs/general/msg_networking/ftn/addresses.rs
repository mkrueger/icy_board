use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{IcyBoard, ftn::FtnAka};
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

/// Every address this board answers to, which `PCBoard` kept under System Address.
pub struct SystemAddresses<'a> {
    insert_table: InsertTable<'a>,
    icy_board: Arc<Mutex<IcyBoard>>,
    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<IcyBoard>>)>>,
    pending_insert: Option<usize>,
    validation_error: Arc<Mutex<Option<String>>>,
}

fn is_usable_aka(address: &EchomailAddress) -> bool {
    address.zone != 0 && address.net != 0 && address.node != 0
}

impl<'a> SystemAddresses<'a> {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let validation_error = Arc::new(Mutex::new(None));
        let content_length = icy_board.lock().unwrap().ftn.akas.len();
        let board = icy_board.clone();
        let insert_table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(content_length),
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("fido_address_header_node")).with_width(24),
                Column::new(get_text("fido_address_header_primary")).with_width(12),
                Column::new(get_text("fido_address_header_domain")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                let board = board.lock().unwrap();
                let Some(aka) = board.ftn.akas.get(*i) else {
                    return Line::from(String::new());
                };
                match j {
                    0 => Line::from(aka.address.to_string()),
                    1 => Line::from(if *i == 0 { "Y" } else { "N" }),
                    2 => Line::from(aka.domain.clone()),
                    _ => Line::from(String::new()),
                }
            }),
            content_length,
        };
        Self {
            insert_table,
            icy_board,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            pending_insert: None,
            validation_error,
        }
    }

    fn open_editor(&mut self, selected: usize) {
        self.edit_config_state = ConfigMenuState::default();
        let board = self.icy_board.lock().unwrap();
        let Some(aka) = board.ftn.akas.get(selected) else {
            return;
        };
        *self.validation_error.lock().unwrap() = (!is_usable_aka(&aka.address)).then(|| get_text("fido_address_invalid"));
        let entry = vec![
            ConfigEntry::Item(
                ListItem::new(get_text("fido_address_node"), ListValue::Text(24, TextFlags::None, aka.address.to_string()))
                    .with_status(get_text("fido_address_node-status"))
                    .with_label_width(10)
                    .with_update_value({
                        let validation_error = self.validation_error.clone();
                        Box::new(move |(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: &ListValue| {
                            let ListValue::Text(_, _, value) = value else {
                                return;
                            };
                            match EchomailAddress::parse(value).filter(is_usable_aka) {
                                Some(address) => {
                                    board.lock().unwrap().ftn.akas[*i].address = address;
                                    *validation_error.lock().unwrap() = None;
                                }
                                None => *validation_error.lock().unwrap() = Some(get_text("fido_address_invalid")),
                            }
                        })
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("fido_address_domain"), ListValue::Text(24, TextFlags::None, aka.domain.clone()))
                    .with_status(get_text("fido_address_domain-status"))
                    .with_label_width(10)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().ftn.akas[*i].domain = value;
                    }),
            ),
        ];
        drop(board);
        self.edit_config = Some(ConfigMenu {
            obj: (selected, self.icy_board.clone()),
            entry,
        });
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected()
            && selected > 0
        {
            self.icy_board.lock().unwrap().ftn.akas.swap(selected, selected - 1);
            self.insert_table.table_state.select(Some(selected - 1));
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected()
            && selected + 1 < self.icy_board.lock().unwrap().ftn.akas.len()
        {
            self.icy_board.lock().unwrap().ftn.akas.swap(selected, selected + 1);
            self.insert_table.table_state.select(Some(selected + 1));
        }
    }
}

impl<'a> Page for SystemAddresses<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        Block::new()
            .title_alignment(Alignment::Center)
            .title(Line::from(Span::from(get_text("fido_address_title")).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding))
            .render(area, frame.buffer_mut());

        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        let sel = self.insert_table.table_state.selected();
        self.insert_table.render_table(frame, area);
        self.insert_table.table_state.select(sel);

        if let Some(edit_config) = &mut self.edit_config {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            Clear.render(area, frame.buffer_mut());
            Block::new()
                .title_alignment(Alignment::Center)
                .title(Line::from(Span::from(get_text("fido_address_editor")).style(get_tui_theme().dialog_box_title)))
                .style(get_tui_theme().dialog_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .render(area, frame.buffer_mut());
            edit_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);
            if let Some(item) = edit_config.get_item(self.edit_config_state.selected) {
                item.text_field_state.set_cursor_position(frame);
            }
        }
    }

    fn request_status(&self) -> ResultState {
        match self.validation_error.lock().unwrap().clone() {
            Some(error) => ResultState::status_line(error),
            None => ResultState::default(),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(edit_config) = &mut self.edit_config {
            let res = edit_config.handle_key_press(key, &mut self.edit_config_state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                if let Some(index) = self.pending_insert.take()
                    && self
                        .icy_board
                        .lock()
                        .unwrap()
                        .ftn
                        .akas
                        .get(index)
                        .is_some_and(|aka| !is_usable_aka(&aka.address))
                {
                    self.icy_board.lock().unwrap().ftn.akas.remove(index);
                    self.insert_table.content_length -= 1;
                }
                *self.validation_error.lock().unwrap() = None;
                self.edit_config = None;
            }
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::PageUp => self.move_up(),
            KeyCode::PageDown => self.move_down(),
            KeyCode::Insert => {
                let index = {
                    let mut board = self.icy_board.lock().unwrap();
                    let index = board.ftn.akas.len();
                    board.ftn.akas.push(FtnAka::default());
                    index
                };
                self.insert_table.content_length += 1;
                self.insert_table.table_state.select(Some(index));
                self.pending_insert = Some(index);
                self.open_editor(index);
            }
            KeyCode::Delete => {
                if let Some(selected) = self.insert_table.table_state.selected()
                    && selected < self.icy_board.lock().unwrap().ftn.akas.len()
                {
                    self.icy_board.lock().unwrap().ftn.akas.remove(selected);
                    self.insert_table.content_length -= 1;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_system_aka_needs_a_zone_net_and_node() {
        assert!(!is_usable_aka(&EchomailAddress::parse("0:0/0").unwrap()));
        assert!(!is_usable_aka(&EchomailAddress::parse("1:2/0").unwrap()));
        assert!(is_usable_aka(&EchomailAddress::parse("1:2/3").unwrap()));
        assert!(is_usable_aka(&EchomailAddress::parse("1:2/3.4").unwrap()));
    }
}
