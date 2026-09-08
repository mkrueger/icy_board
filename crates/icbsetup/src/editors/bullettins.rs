use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        bulletins::{Bullettin, BullettinList},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
};

pub struct BullettinsEditor<'a> {
    path: std::path::PathBuf,
    blt_list: BullettinList,
    orig_blt_list: BullettinList,

    insert_table: InsertTable<'a>,
    sec_levels: Arc<Mutex<Vec<Bullettin>>>,

    detail: super::EditorDialog<(usize, Arc<Mutex<Vec<Bullettin>>>)>,
    save_changes: super::EditorSaveChanges,
}

impl<'a> BullettinsEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let bullettins = if path.exists() {
            BullettinList::load(&path)?
        } else {
            BullettinList::default()
        };
        let command_arc = Arc::new(Mutex::new(bullettins.bullettins.clone()));
        let scroll_state = ScrollbarState::default().content_length(bullettins.bullettins.len());
        let content_length = bullettins.bullettins.len();
        let cmd2 = command_arc.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            columns: vec![Column::new("Bullettin")],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].path.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            blt_list: bullettins.clone(),
            orig_blt_list: bullettins,
            insert_table,
            sec_levels: command_arc,
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }

    fn with_path_base(mut self, path_base: PathBuf) -> Self {
        self.detail.state.path_base = Some(path_base);
        self
    }
}

impl<'a> Page for BullettinsEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = super::list_editor_frame(
            " Bullettins ".to_string(),
            "icb_setup_key_conf_list_help",
            self.detail.is_open() || self.save_changes.is_open(),
        );
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let area = area.inner(Margin { vertical: 8, horizontal: 3 });
            self.detail.render(frame, area, " Edit Bullettin ".to_string(), "");
        }
        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            self.blt_list.bullettins.clear();
            self.blt_list.bullettins.append(&mut self.sec_levels.lock().unwrap().clone());
            crate::editors::save_file(&self.path, || self.blt_list.save(&self.path))
        }) {
            return message;
        }

        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self
                    .save_changes
                    .request_close(self.orig_blt_list.bullettins != *self.sec_levels.lock().unwrap());
            }
            _ => match key.code {
                KeyCode::PageUp => self.insert_table.move_row(&mut self.sec_levels.lock().unwrap(), -1),
                KeyCode::PageDown => self.insert_table.move_row(&mut self.sec_levels.lock().unwrap(), 1),

                KeyCode::Insert => {
                    self.insert_table.push_row(
                        &mut *self.sec_levels.lock().unwrap(),
                        Bullettin {
                            path: PathBuf::new(),
                            required_security: SecurityExpression::default(),
                        },
                    );
                }
                KeyCode::Delete => {
                    self.insert_table.remove_row(&mut *self.sec_levels.lock().unwrap());
                }

                KeyCode::Enter => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.sec_levels.lock().unwrap();
                        let Some(action) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.detail.open(ConfigMenu {
                            obj: (selected_item, self.sec_levels.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new("Path".to_string(), ListValue::Path(action.path.clone()))
                                        .with_label_width(16)
                                        .with_update_path_value(&|board: &(usize, Arc<Mutex<Vec<Bullettin>>>), value: PathBuf| {
                                            board.1.lock().unwrap()[board.0].path = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "Security".to_string(),
                                        ListValue::Text(25, TextFlags::None, action.required_security.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|board: &(usize, Arc<Mutex<Vec<Bullettin>>>), value: String| {
                                            if let Ok(expr) = SecurityExpression::from_str(&value) {
                                                board.1.lock().unwrap()[board.0].required_security = expr;
                                            }
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

pub fn edit_bulletins(board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    let root = board.1.lock().unwrap().root_path.clone();
    PageMessage::OpenSubPage(Box::new(BullettinsEditor::new(&path).unwrap().with_path_base(root)))
}
