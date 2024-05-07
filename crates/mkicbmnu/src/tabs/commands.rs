use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::{config_menu::ResultState, tab_page::TabPage, theme::THEME};
use ratatui::{
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
    Frame,
};

#[derive(Clone, PartialEq)]
pub struct CommandsTab {
    scroll_state: ScrollbarState,
    table_state: TableState,
    commands: Vec<icy_board_engine::icy_board::commands::Command>,
    in_edit_mode: bool,
}

impl CommandsTab {
    pub fn new(menu: Arc<Mutex<Menu>>) -> Self {
        let mnu = menu.lock().unwrap();
        Self {
            scroll_state: ScrollbarState::default().content_length(mnu.commands.len()),
            table_state: TableState::default(),
            commands: mnu.commands.clone(),
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

        let rows = self.commands.iter().enumerate().map(|(i, cmd)| {
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
        if self.commands.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.commands.len() - 1
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
        if self.commands.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i + 1 >= self.commands.len() {
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
        self.commands.push(icy_board_engine::icy_board::commands::Command::default());
        self.scroll_state = self.scroll_state.content_length(self.commands.len());
    }
}

impl TabPage for CommandsTab {
    fn title(&self) -> String {
        "Commands".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(&Margin { vertical: 2, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        /*   if self.in_edit_mode {
            match key.code {
                KeyCode::Esc => {
                    self.in_edit_mode = false;
                    self.write_config();
                    return ResultState::default();
                }
                KeyCode::F(2) => {
                    if let Some(item) = self.conference_config.get_item(self.state.selected) {
                        if item.id == "doors_file" {
                            if let ListValue::Path(path) = &item.value {
                                let path = self.icy_board.lock().unwrap().resolve_file(path);
                                return ResultState {
                                    edit_mode: EditMode::Open("doors_file".to_string(), path.clone()),
                                    status_line: String::new(),
                                };
                            }
                        }
                    }
                }
                _ => {
                    self.conference_config.handle_key_press(key, &mut self.state);
                }
            }
            return ResultState::status_line(String::new());
        }*/
        match key.code {
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('i') | KeyCode::Insert => self.insert(),
            // KeyCode::Char('r') | KeyCode::Delete => self.remove(),
            KeyCode::Char('d') | KeyCode::Enter => {
                if let Some(_state) = self.table_state.selected() {
                    self.in_edit_mode = true;
                    //self.open_editor(state);
                    return ResultState::status_line(String::new());
                } else {
                    self.in_edit_mode = false;
                }
            }
            _ => {}
        }
        ResultState::default()
    }
}
