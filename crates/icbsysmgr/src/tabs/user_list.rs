use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::tab_page::Page;
use icy_board_tui::tab_page::PageMessage;
use icy_board_tui::theme::get_tui_theme;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
};

use super::UserEditor;

pub struct UserList {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,

    in_edit_mode: bool,
}

impl UserList {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            scroll_state: ScrollbarState::default().content_length(icy_board.lock().unwrap().conferences.len()),
            table_state: TableState::default().with_selected(0),
            icy_board: icy_board.clone(),
            in_edit_mode: false,
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, mut area: Rect) {
        area.x += 1;
        area.y += 1;
        area.height -= 1;
        frame.render_stateful_widget(
            Scrollbar::default()
                .style(get_tui_theme().dialog_box_scrollbar)
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
        let header = ["", "Name", "Alias", "Security"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(get_tui_theme().table_header)
            .height(1);

        let l = self.icy_board.lock().unwrap();
        let rows = l.users.iter().enumerate().map(|(i, user)| {
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)).style(get_tui_theme().item),
                Cell::from(user.name.clone()).style(get_tui_theme().item),
                Cell::from(user.alias.clone()).style(get_tui_theme().item),
                Cell::from(user.security_level.to_string()).style(get_tui_theme().item),
            ])
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(3 + 1),
            ],
        )
        .header(header)
        .row_highlight_style(get_tui_theme().selected_item)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
        //.bg(THEME.content.bg.unwrap())
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn prev(&mut self) {
        if self.icy_board.lock().unwrap().conferences.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.icy_board.lock().unwrap().users.len() - 1
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
        if self.icy_board.lock().unwrap().conferences.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i + 1 >= self.icy_board.lock().unwrap().users.len() {
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
        let mut conf = if let Some(i) = self.table_state.selected() {
            self.icy_board.lock().unwrap().conferences[i].clone()
        } else {
            self.icy_board.lock().unwrap().conferences[0].clone()
        };
        conf.name = format!("NewUser{}", self.icy_board.lock().unwrap().conferences.len() + 1);
        self.icy_board.lock().unwrap().conferences.push(conf);
        self.scroll_state = self.scroll_state.content_length(self.icy_board.lock().unwrap().conferences.len());
    }

    fn remove(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if i > 0 {
                self.icy_board.lock().unwrap().conferences.remove(i);
                let len = self.icy_board.lock().unwrap().conferences.len();
                self.scroll_state = self.scroll_state.content_length(len);

                if len >= i - 1 {
                    self.table_state.select(Some(i - 1));
                } else {
                    self.table_state.select(Some(0));
                }
            }
        }
    }
}

impl Page for UserList {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }
    /*
    fn set_cursor_position(&self, frame: &mut Frame) {
        self.conference_config
            .get_item(self.state.selected)
            .unwrap()
            .text_field_state
            .set_cursor_position(frame);
    }

    fn has_control(&self) -> bool {
        self.in_edit_mode
    }*/

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        match key.code {
            KeyCode::Esc => {
                return PageMessage::Close;
            }
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Insert => self.insert(),
            KeyCode::Delete => self.remove(),
            KeyCode::Enter => {
                if let Some(state) = self.table_state.selected() {
                    self.in_edit_mode = true;
                    return PageMessage::OpenSubPage(Box::new(UserEditor::new(self.icy_board.clone(), state)));
                } else {
                    self.in_edit_mode = false;
                }
            }
            _ => {}
        }
        return PageMessage::None;
    }
}
