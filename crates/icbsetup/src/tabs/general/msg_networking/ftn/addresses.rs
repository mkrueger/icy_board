use std::sync::{Arc, Mutex};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{IcyBoard, ftn::FtnAka};
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

/// Every address this board answers to, which `PCBoard` kept under System Address.
pub struct SystemAddresses<'a> {
    insert_table: InsertTable<'a>,
    icy_board: Arc<Mutex<IcyBoard>>,
    detail: crate::editors::EditorDialog<(usize, Arc<Mutex<IcyBoard>>)>,
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
            detail: crate::editors::EditorDialog::default(),
            pending_insert: None,
            validation_error,
        }
    }

    fn open_editor(&mut self, selected: usize) {
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
        self.detail.open(ConfigMenu {
            obj: (selected, self.icy_board.clone()),
            entry,
        });
    }
}

impl<'a> Page for SystemAddresses<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        crate::editors::list_editor_frame(get_text("fido_address_title"), "icb_setup_key_conf_list_help", self.detail.is_open())
            .render(area, frame.buffer_mut());

        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            self.detail.render(frame, area, get_text("fido_address_editor"), "");
        }
    }

    fn request_status(&self) -> ResultState {
        match self.validation_error.lock().unwrap().clone() {
            Some(error) => ResultState::status_line(error),
            None => self.detail.status(),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.detail.handle_key(key) {
            if !self.detail.is_open() {
                if let Some(index) = self.pending_insert.take() {
                    let mut board = self.icy_board.lock().unwrap();
                    if board.ftn.akas.get(index).is_some_and(|aka| !is_usable_aka(&aka.address)) {
                        // Roll back the pending AKA, not an unrelated current selection.
                        board.ftn.akas.remove(index);
                        self.insert_table.sync_rows(board.ftn.akas.len(), self.insert_table.table_state.selected());
                    }
                }
                *self.validation_error.lock().unwrap() = None;
            }
            return message;
        }

        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::PageUp => self.insert_table.move_row(&mut self.icy_board.lock().unwrap().ftn.akas, -1),
            KeyCode::PageDown => self.insert_table.move_row(&mut self.icy_board.lock().unwrap().ftn.akas, 1),
            KeyCode::Insert => {
                let index = {
                    let mut board = self.icy_board.lock().unwrap();
                    let index = board.ftn.akas.len();
                    board.ftn.akas.push(FtnAka::default());
                    self.insert_table.sync_rows(board.ftn.akas.len(), Some(index));
                    index
                };
                self.pending_insert = Some(index);
                self.open_editor(index);
            }
            KeyCode::Delete => {
                self.insert_table.remove_row(&mut self.icy_board.lock().unwrap().ftn.akas);
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
