use std::{io, time::Duration};

use color_eyre::{eyre::Context, Result};
use crossterm::event::{Event, KeyCode};
use icy_board_engine::icy_board::commands::Command;
use icy_board_tui::{app::get_screen_size, term::next_event, theme::THEME, TerminalType};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    text::Span,
    widgets::{block::Title, Block, BorderType, Borders, Padding, Widget},
    Frame, Terminal,
};

pub struct EditCommandDialog {
    _command: Command,
    is_running: bool,
    is_full_screen: bool,
}
impl EditCommandDialog {
    pub(crate) fn new(clone: Command, is_full_screen: bool) -> Self {
        Self {
            _command: clone,
            is_running: true,
            is_full_screen,
        }
    }

    pub(crate) fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        while self.is_running {
            self.draw(terminal)?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        let timeout = Duration::from_secs_f64(1.0 / 50.0);
        match next_event(timeout)? {
            Some(Event::Key(key)) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.is_running = false,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn draw(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        terminal
            .draw(|frame| {
                let screen = get_screen_size(frame, self.is_full_screen);
                self.ui(frame, screen);
            })
            .wrap_err("terminal.draw")?;
        Ok(())
    }

    fn ui(&self, frame: &mut Frame, screen: Rect) {
        let area = screen;
        let block = Block::new()
            .title(Title::from(Span::from(" Command ID 1 ").style(THEME.content_box_title)).alignment(Alignment::Center))
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
    }
}
