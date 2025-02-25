pub mod conference_editor;
pub use conference_editor::*;
use icy_board_tui::BORDER_SET;
use icy_board_tui::get_text;
use icy_board_tui::tab_page::Page;
use icy_board_tui::tab_page::PageMessage;
use ratatui::text::Line;
use ratatui::text::Span;

use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::theme::get_tui_theme;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
};

pub struct ConferenceListEditor {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl ConferenceListEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            scroll_state: ScrollbarState::default().content_length(icy_board.lock().unwrap().conferences.len()),
            table_state: TableState::default().with_selected(0),
            icy_board: icy_board.clone(),
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, mut area: Rect) {
        area.x += 2;
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
        let l = self.icy_board.lock().unwrap();
        log::info!("Conferences: {:?}", l.conferences.len());
        let mut rows = l
            .conferences
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, cmd)| Row::new(vec![Cell::from(format!("{:-3})", i)), Cell::from(cmd.name.clone())]))
            .collect::<Vec<_>>();
        rows.push(Row::new(vec![Cell::from(format!("{:-3})", rows.len() + 1)), Cell::from("")]).style(get_tui_theme().table_inactive));

        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
            ],
        )
        .row_highlight_style(get_tui_theme().selected_item)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
        .style(get_tui_theme().table)
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
        let mut conf = if let Some(i) = self.table_state.selected() {
            self.icy_board.lock().unwrap().conferences[i].clone()
        } else {
            self.icy_board.lock().unwrap().conferences[0].clone()
        };
        conf.name = format!("New Conference #{}", self.icy_board.lock().unwrap().conferences.len() + 1);
        self.icy_board.lock().unwrap().conferences.push(conf);
        self.scroll_state = self.scroll_state.content_length(self.icy_board.lock().unwrap().conferences.len());
    }

    fn remove(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(state) = self.table_state.selected() {
                if state + 1 >= self.icy_board.lock().unwrap().conferences.len() {
                    return;
                }
            }

            if i > 0 {
                self.icy_board.lock().unwrap().conferences.remove(i + 1);
                let len = self.icy_board.lock().unwrap().conferences.len();
                self.scroll_state = self.scroll_state.content_length(len);
            }
        }
    }
}

impl Page for ConferenceListEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let disp_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        Clear.render(disp_area, frame.buffer_mut());

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(disp_area, frame.buffer_mut());

        let val = get_text("conf_list_title");
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().menu_title).render(
            Rect {
                x: disp_area.x + 1 + disp_area.width.saturating_sub(width) / 2,
                y: disp_area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        frame.buffer_mut().set_string(
            disp_area.x + 1,
            disp_area.y + 2,
            "─".repeat((disp_area.width as usize).saturating_sub(2)),
            get_tui_theme().dialog_box,
        );

        let area = Rect {
            x: disp_area.x + 1,
            y: area.y + 4,
            width: disp_area.width - 3,
            height: area.height - 5,
        };
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        match key.code {
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Insert => self.insert(),
            KeyCode::Delete => self.remove(),
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::PageDown => {
                if let Some(state) = self.table_state.selected() {
                    if state + 2 < self.icy_board.lock().unwrap().conferences.len() {
                        self.icy_board.lock().unwrap().conferences.swap(state + 2, state + 1);
                        self.table_state.select(Some(state + 1));
                    }
                }
            }
            KeyCode::PageUp => {
                if let Some(state) = self.table_state.selected() {
                    if state >= 1 && state + 1 < self.icy_board.lock().unwrap().conferences.len() {
                        self.icy_board.lock().unwrap().conferences.swap(state, state + 1);
                        self.table_state.select(Some(state - 1));
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(state) = self.table_state.selected() {
                    if state + 1 < self.icy_board.lock().unwrap().conferences.len() {
                        return PageMessage::OpenSubPage(Box::new(ConferenceEditor::new(self.icy_board.clone(), state + 1)));
                    } else {
                        let _ = std::io::stdout().write(&[0x07]);
                    }
                }
            }
            _ => {}
        }
        PageMessage::None
    }
}
