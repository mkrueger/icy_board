use std::sync::Arc;

use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::{theme::THEME, TerminalType};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
    Frame, Terminal,
};

use crate::edit_command_dialog::EditCommandDialog;

use super::TabPage;

#[derive(Clone, PartialEq)]
pub struct CommandsTab {
    scroll_state: ScrollbarState,
    table_state: TableState,
    commands: Vec<icy_board_engine::icy_board::commands::Command>,
}

impl CommandsTab {
    pub fn new(menu: Arc<Menu>) -> Self {
        Self {
            scroll_state: ScrollbarState::default(),
            table_state: TableState::default(),
            commands: menu.commands.clone(),
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(&Margin { vertical: 1, horizontal: 1 }),
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
                Cell::from(format!("{})", i + 1)),
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
}

impl TabPage for CommandsTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(&Margin { vertical: 2, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn insert(&mut self) {
        self.commands.push(icy_board_engine::icy_board::commands::Command::default());
    }
    fn prev(&mut self) {
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
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.commands.len() - 1 {
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

    fn request_edit_mode(&mut self, terminal: &mut TerminalType) -> Option<(u16, u16)> {
        if let Some(sel) = self.table_state.selected() {
            let cmd = &self.commands[sel];
            EditCommandDialog::new(cmd.clone(), false).run(terminal).unwrap();
        }
        None
    }
}
