use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{menu::Menu, IcyBoard};
use icy_board_tui::{config_menu::ResultState, insert_table::InsertTable, pcb_line::get_styled_pcb_line, tab_page::TabPage, theme::get_tui_theme};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
    Frame,
};

use crate::edit_command_dialog::EditCommandDialog;

pub struct CommandsTab<'a> {
    menu: Arc<Mutex<Menu>>,
    insert_table: InsertTable<'a>,
    edit_cmd_dialog: Option<EditCommandDialog<'a>>,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl<'a> CommandsTab<'a> {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>) -> Self {
        let len = menu.lock().unwrap().commands.len().max(1);
        let mnu2 = menu.clone();
        let insert_table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(len),
            table_state: TableState::default().with_selected(0),
            headers: vec!["   ".to_string(), "Keyword".to_string(), "Display".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= mnu2.lock().unwrap().commands.len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", i + 1)),
                    1 => Line::from(mnu2.lock().unwrap().commands[*i].keyword.clone()),
                    2 => get_styled_pcb_line(&mnu2.lock().unwrap().commands[*i].display),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length: len,
        };

        Self {
            insert_table,
            menu,
            edit_cmd_dialog: None,
            icy_board,
        }
    }
    fn insert(&mut self) {
        self.menu
            .lock()
            .unwrap()
            .commands
            .push(icy_board_engine::icy_board::commands::Command::default());
        self.insert_table.scroll_state = self.insert_table.scroll_state.content_length(self.menu.lock().unwrap().commands.len());
        self.insert_table.content_length += 1;
    }

    fn remove(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            self.menu.lock().unwrap().commands.remove(selected);
            let len = self.menu.lock().unwrap().commands.len();
            if len > 0 {
                self.insert_table.table_state.select(Some(selected.min(len - 1)))
            } else {
                self.insert_table.table_state.select(None)
            }
            self.insert_table.content_length -= 1;
        }
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected > 0 {
                let mut menu = self.menu.lock().unwrap();
                menu.commands.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.menu.lock().unwrap().commands.len() {
                let mut menu = self.menu.lock().unwrap();
                menu.commands.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> TabPage for CommandsTab<'a> {
    fn title(&self) -> String {
        "Commands".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(dialog) = &mut self.edit_cmd_dialog {
            dialog.ui(frame, area);
            return;
        }
        let area = area.inner(Margin::new(2, 2));

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(Margin::new(1, 1));
        self.insert_table.render_table(frame, area);
    }

    fn has_control(&self) -> bool {
        self.edit_cmd_dialog.is_some()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if let Some(dialog) = &mut self.edit_cmd_dialog {
            if let Ok(false) = dialog.handle_key_press(key) {
                if let Some(selected) = self.insert_table.table_state.selected() {
                    self.menu.lock().unwrap().commands[selected] = dialog.command.lock().unwrap().clone();
                }
                self.edit_cmd_dialog = None;
                return ResultState::default();
            }
            return ResultState::status_line(String::new());
        }
        match key.code {
            KeyCode::Char('1') => self.move_up(),
            KeyCode::Char('2') => self.move_down(),
            KeyCode::Char('i') | KeyCode::Insert => self.insert(),
            KeyCode::Char('r') | KeyCode::Delete => self.remove(),

            KeyCode::Char('d') | KeyCode::Enter => {
                if let Some(selected) = self.insert_table.table_state.selected() {
                    let cmd = self.menu.lock().unwrap().commands[selected].clone();
                    let m = self.menu.clone();
                    self.edit_cmd_dialog = Some(EditCommandDialog::new(self.icy_board.clone(), m, cmd, selected + 1));
                    return ResultState::status_line(String::new());
                }
            }
            _ => {
                let _ = self.insert_table.handle_key_press(key);
            }
        }
        ResultState::default()
    }
}
