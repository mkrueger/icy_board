use std::{
    io::{self, stdout},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use dizbase::file_base::FileBase;
use ratatui::{
    prelude::*,
    widgets::{
        block::Title,
        canvas::{Canvas, Rectangle},
        *,
    },
};

pub struct FileBaseViewer {
    file_base: FileBase,
}


impl FileBaseViewer {
    pub fn new(file_base: FileBase) -> Self {
        Self {
            file_base
        }
    }

    pub fn run(&mut self) -> Res<()> {
        let mut terminal = init_terminal()?;
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(16);

        loop {
            terminal.draw(|frame| self.ui(frame))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                return Ok(());
                            }
                            KeyCode::Down | KeyCode::Char('s') => self.y = (self.y + 1).min(1),
                            KeyCode::Up | KeyCode::Char('w') => self.y = (self.y - 1).max(0),
                            KeyCode::Right | KeyCode::Char('d') => self.x = (self.x + 1).min(2),
                            KeyCode::Left | KeyCode::Char('a') => self.x = (self.x - 1).max(0),
                            KeyCode::Enter => {
                                self.selected = Some(Instant::now());
                            }
                            _ => {}
                        }
                    }
                }
            }

            if let Some(selected) = self.selected {
                if selected.elapsed() >= Duration::from_millis(500) {
                    return Ok(self.buttons[(self.y * 3 + self.x) as usize].message);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                //     self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&self, frame: &mut Frame) {
        frame.render_widget(self.main_canvas(frame.size()), frame.size());
    }
}

fn init_terminal() -> crate::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
