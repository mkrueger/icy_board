use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    ftn::freq::{FreqMagic, FreqPath},
};
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

/// The three FREQ lists differ only in their columns and in the fields the
/// editor offers, so the frame around them is written once.
macro_rules! freq_list {
    ($name:ident, $title:expr, $editor_title:expr, $list:ident, $new:expr, $columns:expr, $column:expr, $entries:expr) => {
        pub struct $name<'a> {
            table: InsertTable<'a>,
            board: Arc<Mutex<IcyBoard>>,
            detail: crate::editors::EditorDialog<(usize, Arc<Mutex<IcyBoard>>)>,
        }

        impl<'a> $name<'a> {
            pub fn new(board: Arc<Mutex<IcyBoard>>) -> Self {
                let content_length = board.lock().unwrap().ftn.freq.$list.len();
                let content = board.clone();
                Self {
                    table: InsertTable {
                        scroll_state: ScrollbarState::default().content_length(content_length),
                        table_state: TableState::default().with_selected(0),
                        columns: $columns(),
                        numbered: true,
                        get_content: Box::new(move |_table, i, j| {
                            let board = content.lock().unwrap();
                            match board.ftn.freq.$list.get(*i) {
                                Some(entry) => Line::from($column(entry, *j)),
                                None => Line::from(String::new()),
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
                self.detail.state.path_base = Some(board.root_path.clone());
                let Some(entry) = board.ftn.freq.$list.get(selected) else {
                    return;
                };
                let entry = $entries(entry);
                drop(board);
                self.detail.open(ConfigMenu {
                    obj: (selected, self.board.clone()),
                    entry,
                });
            }
        }

        impl Page for $name<'_> {
            fn render(&mut self, frame: &mut Frame, area: Rect) {
                Clear.render(area, frame.buffer_mut());
                crate::editors::list_editor_frame(get_text($title), "icb_setup_key_conf_list_help", self.detail.is_open()).render(area, frame.buffer_mut());

                let area = area.inner(Margin { horizontal: 1, vertical: 1 });
                self.table.render_list(frame, area);

                if self.detail.is_open() {
                    let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
                    area.height += 1;
                    self.detail.render(frame, area, get_text($editor_title), "");
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
                        self.table.push_row(&mut self.board.lock().unwrap().ftn.freq.$list, $new);
                    }
                    KeyCode::Delete => {
                        self.table.remove_row(&mut self.board.lock().unwrap().ftn.freq.$list);
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
    };
}

freq_list!(
    FreqPathList,
    "fido_freq_path_title",
    "fido_freq_path_editor",
    paths,
    FreqPath::default(),
    || vec![
        Column::new(get_text("fido_freq_header_path")).with_width(57),
        Column::new(get_text("fido_freq_header_password")),
    ],
    |entry: &FreqPath, column: usize| match column {
        0 => entry.path.display().to_string(),
        1 => entry.password.clone(),
        _ => String::new(),
    },
    |entry: &FreqPath| vec![
        ConfigEntry::Item(
            ListItem::new(get_text("fido_freq_path"), ListValue::Path(entry.path.clone()))
                .with_status(get_text("fido_freq_path-status"))
                .with_help(get_text("fido_freq_path-help"))
                .with_label_width(14)
                .with_update_path_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                    board.lock().unwrap().ftn.freq.paths[*i].path = value;
                }),
        ),
        ConfigEntry::Item(
            ListItem::new(get_text("fido_freq_password"), ListValue::Text(10, TextFlags::None, entry.password.clone()),)
                .with_status(get_text("fido_freq_password-status"))
                .with_help(get_text("fido_freq_password-help"))
                .with_label_width(14)
                .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                    board.lock().unwrap().ftn.freq.paths[*i].password = value;
                }),
        ),
    ]
);

freq_list!(
    FreqMagicNames,
    "fido_freq_magic_title",
    "fido_freq_magic_editor",
    magic,
    FreqMagic::default(),
    || vec![
        Column::new(get_text("fido_freq_header_magic")).with_width(22),
        Column::new(get_text("fido_freq_header_file")).with_width(31),
        Column::new(get_text("fido_freq_header_password")),
    ],
    |entry: &FreqMagic, column: usize| match column {
        0 => entry.name.clone(),
        1 => entry.file.display().to_string(),
        2 => entry.password.clone(),
        _ => String::new(),
    },
    |entry: &FreqMagic| vec![
        ConfigEntry::Item(
            ListItem::new(get_text("fido_freq_magic"), ListValue::Text(20, TextFlags::None, entry.name.clone()))
                .with_status(get_text("fido_freq_magic-status"))
                .with_help(get_text("fido_freq_magic-help"))
                .with_label_width(14)
                .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                    board.lock().unwrap().ftn.freq.magic[*i].name = value.to_ascii_uppercase();
                }),
        ),
        ConfigEntry::Item(
            ListItem::new(get_text("fido_freq_file"), ListValue::Path(entry.file.clone()))
                .with_status(get_text("fido_freq_file-status"))
                .with_label_width(14)
                .with_update_path_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: PathBuf| {
                    board.lock().unwrap().ftn.freq.magic[*i].file = value;
                }),
        ),
        ConfigEntry::Item(
            ListItem::new(get_text("fido_freq_password"), ListValue::Text(10, TextFlags::None, entry.password.clone()),)
                .with_status(get_text("fido_freq_password-status"))
                .with_label_width(14)
                .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                    board.lock().unwrap().ftn.freq.magic[*i].password = value;
                }),
        ),
    ]
);

freq_list!(
    FreqDenyList,
    "fido_freq_deny_title",
    "fido_freq_deny_editor",
    deny,
    EchomailAddress::default(),
    || vec![Column::new(get_text("fido_freq_header_node"))],
    |entry: &EchomailAddress, column: usize| match column {
        0 => entry.to_string(),
        _ => String::new(),
    },
    |entry: &EchomailAddress| vec![ConfigEntry::Item(
        ListItem::new(get_text("fido_freq_node"), ListValue::Text(24, TextFlags::None, entry.to_string()))
            .with_status(get_text("fido_freq_node-status"))
            .with_help(get_text("fido_freq_node-help"))
            .with_label_width(14)
            .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                if let Some(address) = EchomailAddress::parse(&value) {
                    board.lock().unwrap().ftn.freq.deny[*i] = address;
                }
            }),
    )]
);
