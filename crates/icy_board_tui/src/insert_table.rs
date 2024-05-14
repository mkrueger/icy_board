use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::Res;
use ratatui::{
    layout::{Constraint, Rect},
    text::{Line, Text},
    widgets::{Cell, HighlightSpacing, Row, ScrollbarState, Table, TableState},
    Frame,
};

use crate::theme::THEME;

pub struct InsertTable<'a> {
    pub scroll_state: ScrollbarState,
    pub table_state: TableState,
    pub headers: Vec<String>,

    pub get_content: Box<dyn Fn(&InsertTable, &usize, &usize) -> Line<'a>>,
    pub content_length: usize,
}

impl<'a> InsertTable<'a> {
    pub fn render_table(&mut self, frame: &mut Frame, mut area: Rect) {
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
        let mut widths = Vec::new();
        for i in 0..self.headers.len() - 1 {
            widths.push(Constraint::Length(self.headers[i].len() as u16 + 2));
        }
        widths.push(Constraint::Min(25 + 1));
        let table = Table::new(rows, widths)
            .header(header)
            .highlight_style(THEME.selected_item)
            .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
            .style(THEME.table)
            .highlight_spacing(HighlightSpacing::Always);
        area.width -= 1;
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> Res<()> {
        match key.code {
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
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

    fn page_up(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn page_down(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => (i + 10).min(self.content_length - 1),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn home(&mut self) {
        if self.content_length == 0 {
            return;
        }
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    fn end(&mut self) {
        if self.content_length == 0 {
            return;
        }
        self.table_state.select(Some(self.content_length - 1));
        self.scroll_state = self.scroll_state.position(self.content_length);
    }
}
