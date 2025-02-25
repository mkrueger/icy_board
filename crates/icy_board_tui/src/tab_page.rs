use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect, text::Text};

use crate::config_menu::ResultState;

pub enum PageMessage {
    None,
    Close,
    ResultState(ResultState),
    OpenSubPage(Box<dyn Page>),
    ExternalProgramStarted,
}

pub trait Page {
    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn request_status(&self) -> ResultState {
        ResultState::default()
    }

    fn handle_key_press(&mut self, _key: KeyEvent) -> PageMessage {
        PageMessage::None
    }
}

pub trait TabPage {
    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn handle_key_press(&mut self, _key: KeyEvent) -> ResultState {
        ResultState::default()
    }

    fn request_status(&self) -> ResultState {
        ResultState::default()
    }

    fn set_cursor_position(&self, _frame: &mut Frame) {}

    fn has_control(&self) -> bool {
        false
    }

    fn title(&self) -> String;

    fn get_help(&self) -> Text<'static> {
        Text::from("")
    }

    fn is_dirty(&self) -> bool {
        false
    }
}
