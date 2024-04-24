use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{config_menu::ResultState, theme::THEME, TerminalType};
use ratatui::text::Span;
use ratatui::{
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
    Frame,
};

use super::TabPage;

#[derive(Clone)]
pub struct ConferencesTab {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,

    in_edit_mode: bool,
}

impl ConferencesTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            scroll_state: ScrollbarState::default().content_length(icy_board.lock().unwrap().conferences.len()),
            table_state: TableState::default(),
            icy_board: icy_board.clone(),
            in_edit_mode: false,
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

        let l = self.icy_board.lock().unwrap();
        let rows = l
            .conferences
            .iter()
            .enumerate()
            .map(|(i, cmd)| Row::new(vec![Cell::from(format!("{:-3})", i + 1)), Cell::from(cmd.name.clone())]));
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
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

    fn prev(&mut self) {
        if self.icy_board.lock().unwrap().conferences.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.icy_board.lock().unwrap().conferences.len() - 1
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
                if i + 1 >= self.icy_board.lock().unwrap().conferences.len() {
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
        //  self.icy_board.conferences.push(icy_board_engine::icy_board::commands::Command::default());
        self.scroll_state = self.scroll_state.content_length(self.icy_board.lock().unwrap().conferences.len());
    }

    fn draw_label(&self, label: &str, line: &mut Rect, len: u16, frame: &mut Frame) {
        Text::from(label).style(THEME.item).render(*line, frame.buffer_mut());
        line.x += len;
        line.width -= len;
        Span::from(":").style(THEME.item).render(*line, frame.buffer_mut());
        line.x += 2;
        line.width -= 2;
    }

    fn render_editor(&self, frame: &mut Frame, mut area: Rect) {
        let Ok(mut board) = self.icy_board.lock() else {
            return;
        };
        let cur_conf = board.conferences.get_mut(self.table_state.selected().unwrap()).unwrap();
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label(&format!("Name (#{})", self.table_state.selected().unwrap() + 1), &mut line, 14, frame);
        Text::from(cur_conf.name.to_string()).style(THEME.value).render(line, frame.buffer_mut());

        area.y += 1;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };

        let l1 = 28;

        self.draw_label("Public Conference", &mut line, l1, frame);
        Text::from(if cur_conf.is_public { "Y" } else { "N" })
            .style(THEME.value)
            .render(line, frame.buffer_mut());
        line.x += 15;
        line.x -= 15;
        self.draw_label("Req. Security if Public", &mut line, 24, frame);
        Text::from(cur_conf.required_security.level.to_string())
            .style(THEME.value)
            .render(line, frame.buffer_mut());
        area.y += 1;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label("Password to Join if Private", &mut line, l1, frame);
        Text::from(cur_conf.password.to_string()).style(THEME.value).render(line, frame.buffer_mut());
        line.x += 20;

        area.y += 2;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label("Name/Loc of User's Menu", &mut line, l1, frame);
        Text::from(cur_conf.users_menu.to_string_lossy())
            .style(THEME.value)
            .render(line, frame.buffer_mut());

        area.y += 1;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label("Name/Loc of Sysop's Menu", &mut line, l1, frame);
        Text::from(cur_conf.sysop_menu.to_string_lossy())
            .style(THEME.value)
            .render(line, frame.buffer_mut());

        area.y += 1;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label("Name/Loc of NEWS File", &mut line, l1, frame);
        Text::from(cur_conf.news_file.to_string_lossy())
            .style(THEME.value)
            .render(line, frame.buffer_mut());

        area.y += 1;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label("Name/Loc of Conf INTRO File", &mut line, l1, frame);
        Text::from(cur_conf.intro_file.to_string_lossy())
            .style(THEME.value)
            .render(line, frame.buffer_mut());

        area.y += 1;
        let mut line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.draw_label("Location for Attachments", &mut line, l1, frame);
        Text::from(cur_conf.attachment_location.to_string_lossy())
            .style(THEME.value)
            .render(line, frame.buffer_mut());
    }
}

impl TabPage for ConferencesTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(&Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        if self.in_edit_mode {
            self.render_editor(frame, area);
            return;
        }

        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        match key.code {
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => self.prev(),
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => self.next(),
            crossterm::event::KeyCode::Char('i') | crossterm::event::KeyCode::Insert => self.insert(),
            _ => {}
        }

        ResultState::default()
    }

    fn request_edit_mode(&mut self, _terminal: &mut TerminalType, _full_screen: bool) -> ResultState {
        if let Some(_state) = self.table_state.selected() {
            self.in_edit_mode = true;
            ResultState {
                in_edit_mode: true,
                status_line: String::new(),
            }
        } else {
            self.in_edit_mode = false;
            ResultState::default()
        }
    }
}
