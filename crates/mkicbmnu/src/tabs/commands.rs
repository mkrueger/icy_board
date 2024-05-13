use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{icy_board::menu::Menu, Res};
use icy_board_tui::{config_menu::ResultState, tab_page::TabPage, theme::THEME};
use ratatui::{
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
    Frame,
};

pub struct InsertTable {
    pub scroll_state: ScrollbarState,
    pub table_state: TableState,
    pub headers: Vec<String>,

    pub get_content: Box<dyn Fn(&InsertTable, &usize, &usize) -> String>,
    pub content_length: usize,
}

impl InsertTable {
    pub fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = self
            .headers
            .iter()
            .cloned()
            .map(Cell::from)
            .collect::<Row>()
            .style(THEME.table_header)
            .height(1);

        let mut rows = Vec::new();
        for i in 0..self.content_length {
            let mut row = Vec::new();
            for j in 0..self.headers.len() {
                row.push(Cell::from((self.get_content)(self, &i, &j)));
            }
            rows.push(Row::new(row));
        }
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(self.headers[0].len() as u16 + 2),
                Constraint::Min(25 + 1),
            ],
        )
        .header(header)
        .highlight_style(THEME.selected_item)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
        //.bg(THEME.content.bg.unwrap())
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    pub(crate) fn handle_key_press(&mut self, key: KeyEvent) -> Res<()> {
        match key.code {
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            //KeyCode::Char('i') | KeyCode::Insert => self.insert(),
            // KeyCode::Char('r') | KeyCode::Delete => self.remove(),
            // KeyCode::Char('d') | KeyCode::Enter => self.edit(),
            _ => {}
        }
        Ok(())
    }

    fn prev(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i + 1 < self.content_length {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }
}

use crate::edit_command_dialog::EditCommandDialog;

pub struct CommandsTab {
    menu: Arc<Mutex<Menu>>,
    scroll_state: ScrollbarState,
    table_state: TableState,
    edit_cmd_dialog: Option<EditCommandDialog>,
}

impl CommandsTab {
    pub fn new(menu: Arc<Mutex<Menu>>) -> Self {
        let len = menu.lock().unwrap().commands.len();
        Self {
            scroll_state: ScrollbarState::default().content_length(len),
            table_state: TableState::default(),
            menu,
            edit_cmd_dialog: None,
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, mut area: Rect) {
        area.y += 1;
        area.height -= 1;
        frame.render_stateful_widget(
            Scrollbar::default()
                .style(THEME.content_box)
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .thumb_symbol("█")
                .track_symbol(Some("░"))
                .end_symbol(Some("▼")),
            area,
            &mut self.scroll_state,
        );
    }
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["", "Keyword", "Display"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(THEME.table_header)
            .height(1);
        let l = self.menu.lock().unwrap();
        let rows = l.commands.iter().enumerate().map(|(i, cmd)| {
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)),
                Cell::from(cmd.keyword.clone()),
                Cell::from(cmd.display.clone()),
            ])
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(25),
            ],
        )
        .header(header)
        .highlight_style(THEME.selected_item)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
        //.bg(THEME.content.bg.unwrap())
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn prev(&mut self) {
        if self.menu.lock().unwrap().commands.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.menu.lock().unwrap().commands.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn next(&mut self) {
        if self.menu.lock().unwrap().commands.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i + 1 >= self.menu.lock().unwrap().commands.len() {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn insert(&mut self) {
        self.menu
            .lock()
            .unwrap()
            .commands
            .push(icy_board_engine::icy_board::commands::Command::default());
        self.scroll_state = self.scroll_state.content_length(self.menu.lock().unwrap().commands.len());
    }
}

impl TabPage for CommandsTab {
    fn title(&self) -> String {
        "Commands".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(dialog) = &mut self.edit_cmd_dialog {
            dialog.ui(frame, area);
            return;
        }
        let area = area.inner(&Margin { vertical: 2, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn has_control(&self) -> bool {
        self.edit_cmd_dialog.is_some()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if let Some(dialog) = &mut self.edit_cmd_dialog {
            if let Ok(false) = dialog.handle_key_press(key) {
                if let Some(selected) = self.table_state.selected() {
                    self.menu.lock().unwrap().commands[selected] = dialog.command.lock().unwrap().clone();
                }
                self.edit_cmd_dialog = None;
                return ResultState::default();
            }
            return ResultState::status_line(String::new());
        }
        match key.code {
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('i') | KeyCode::Insert => self.insert(),
            // KeyCode::Char('r') | KeyCode::Delete => self.remove(),
            KeyCode::Char('d') | KeyCode::Enter => {
                if let Some(selected) = self.table_state.selected() {
                    self.edit_cmd_dialog = Some(EditCommandDialog::new(self.menu.lock().unwrap().commands[selected].clone()));
                    return ResultState::status_line(String::new());
                }
            }
            _ => {}
        }
        ResultState::default()
    }
}
