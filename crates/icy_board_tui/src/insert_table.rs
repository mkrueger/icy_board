use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::Res;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    text::{Line, Text},
    widgets::{Cell, HighlightSpacing, Row, ScrollbarState, Table, TableState},
};

use crate::theme::get_tui_theme;

/// Width of the record number column, matching PCBoard's `"%3ld)"` records.
const NUMBER_WIDTH: u16 = 5;

/// A list column. PCBSETUP places its columns at fixed offsets rather than letting them
/// hug their heading, so a column may state how far it reaches to the next one.
pub struct Column {
    title: String,
    width: Option<u16>,
}

impl Column {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: None,
        }
    }

    pub fn with_width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    fn advance(&self) -> u16 {
        self.width.unwrap_or(self.title.chars().count() as u16 + 2)
    }
}

pub struct InsertTable<'a> {
    pub scroll_state: ScrollbarState,
    pub table_state: TableState,
    pub columns: Vec<Column>,
    /// PCBoard numbers the records of every list editor except the ones keyed by a letter.
    pub numbered: bool,

    pub get_content: Box<dyn Fn(&InsertTable, &usize, &usize) -> Line<'a>>,
    pub content_length: usize,
}

impl<'a> InsertTable<'a> {
    pub fn render_table(&mut self, frame: &mut Frame, mut area: Rect) {
        let mut header_cells = Vec::new();
        if self.numbered {
            header_cells.push(Cell::from(""));
        }
        for column in &self.columns {
            let title = column.title.trim_end();
            let underline = "═".repeat(title.chars().count());
            header_cells.push(Cell::from(Text::from(vec![Line::from(title.to_string()), Line::from(underline)])));
        }
        let header = Row::new(header_cells).style(get_tui_theme().table_header).height(2);

        let mut rows = Vec::new();
        for i in 0..self.content_length {
            let mut row = Vec::new();
            if self.numbered {
                row.push(Cell::from(Line::from(format!("{:>3}) ", i + 1))));
            }
            for j in 0..self.columns.len() {
                row.push(Cell::from((self.get_content)(self, &i, &j)));
            }
            rows.push(Row::new(row));
        }
        let mut widths = Vec::new();
        if self.numbered {
            widths.push(Constraint::Length(NUMBER_WIDTH));
        }
        for column in self.columns.iter().take(self.columns.len().saturating_sub(1)) {
            widths.push(Constraint::Length(column.advance()));
        }
        widths.push(Constraint::Min(self.columns.last().map_or(26, Column::advance)));
        let table = Table::new(rows, widths)
            .header(header)
            // A width is the distance to the next column, the way PCBSETUP lays its lists out.
            .column_spacing(0)
            .row_highlight_style(get_tui_theme().selected_item)
            .style(get_tui_theme().table)
            // The highlighted row is marked by its colour, so no gutter is reserved for a symbol.
            .highlight_spacing(HighlightSpacing::Never);
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
        self.scroll_state = self.scroll_state.position(i);
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
        self.scroll_state = self.scroll_state.position(i);
    }

    fn page_up(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn page_down(&mut self) {
        if self.content_length == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 10).min(self.content_length - 1),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn render(table: &mut InsertTable) -> Vec<String> {
        render_wide(table, 40)
    }

    fn render_wide(table: &mut InsertTable, width: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, 8)).unwrap();
        terminal.draw(|frame| table.render_table(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn table(numbered: bool) -> InsertTable<'static> {
        InsertTable {
            scroll_state: ScrollbarState::default().content_length(2),
            table_state: TableState::default(),
            columns: vec![Column::new("Path"), Column::new("Password")],
            numbered,
            get_content: Box::new(|_table, i, j| Line::from(format!("r{i}c{j}"))),
            content_length: 2,
        }
    }

    #[test]
    fn column_headers_are_underlined_like_pcbsetup() {
        let lines = render(&mut table(true));
        assert!(lines[0].contains("Path"), "{lines:?}");
        assert!(lines[0].contains("Password"), "{lines:?}");
        assert!(lines[1].contains("════"), "expected an underline row, got {lines:?}");
        assert!(lines[1].contains("════════"), "underline should span the header text, got {lines:?}");
    }

    /// PCBSETUP's magic name editor puts its columns 22 and 31 characters apart
    /// instead of letting each one hug its heading.
    #[test]
    fn a_column_width_spaces_the_next_column_out() {
        let mut table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(1),
            table_state: TableState::default(),
            columns: vec![
                Column::new("Magic Name").with_width(22),
                Column::new("File").with_width(31),
                Column::new("Password"),
            ],
            numbered: true,
            get_content: Box::new(|_table, _i, j| Line::from(["magic", "file", "secret"][*j])),
            content_length: 1,
        };
        let lines = render_wide(&mut table, 78);

        let heading = &lines[0];
        assert_eq!(heading.find("Magic Name"), Some(5), "{lines:?}");
        assert_eq!(heading.find("File"), Some(5 + 22), "{lines:?}");
        assert_eq!(heading.find("Password"), Some(5 + 22 + 31), "{lines:?}");
        assert_eq!(lines[2].find("secret"), Some(5 + 22 + 31), "records line up under their heading, {lines:?}");
    }

    #[test]
    fn records_are_numbered_by_the_shared_control() {
        let lines = render(&mut table(true));
        assert!(lines[2].contains("1)"), "{lines:?}");
        assert!(lines[3].contains("2)"), "{lines:?}");
        assert!(lines[2].contains("r0c0"), "{lines:?}");
    }

    #[test]
    fn numbering_can_be_turned_off() {
        let lines = render(&mut table(false));
        assert!(!lines[2].contains("1)"), "{lines:?}");
        assert!(lines[2].contains("r0c0"), "{lines:?}");
    }
}
