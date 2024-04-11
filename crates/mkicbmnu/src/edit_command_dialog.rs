use std::{
    io::{self, stdout},
    time::Duration,
};

use color_eyre::{eyre::Context, Result};
use crossterm::{
    event::{Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use icy_board_engine::icy_board::commands::Command;
use icy_board_tui::{term::next_event, theme::THEME, TerminalType};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Padding, Widget},
    Frame, Terminal, TerminalOptions, Viewport,
};

pub struct EditCommandDialog {
    command: Command,
    is_running: bool,
    is_full_screen: bool,
}
impl EditCommandDialog {
    pub(crate) fn new(clone: Command, is_full_screen: bool) -> Self {
        Self {
            command: clone,
            is_running: true,
            is_full_screen,
        }
    }

    pub(crate) fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        let options = TerminalOptions {
            viewport: Viewport::Fullscreen,
        };

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
                let screen = if self.is_full_screen {
                    frame.size()
                } else {
                    let width = frame.size().width.min(80);
                    let height = frame.size().height.min(25);

                    let x = frame.size().x + (frame.size().width - width) / 2;
                    let y = frame.size().y + (frame.size().height - height) / 2;
                    Rect::new(frame.size().x + x, frame.size().y + y, width, height)
                };
                self.ui(frame, screen);
            })
            .wrap_err("terminal.draw")?;
        Ok(())
    }

    fn ui(&self, frame: &mut Frame, screen: Rect) {
        let area = frame.size().inner(&Margin { vertical: 5, horizontal: 5 });
        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        let area2 = area.inner(&Margin { vertical: 2, horizontal: 2 });

        block.render(area, frame.buffer_mut());
    }
}
