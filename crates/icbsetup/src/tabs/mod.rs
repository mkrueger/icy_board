pub mod about;

pub use about::*;

pub mod general;
use crossterm::event::KeyEvent;
pub use general::*;

pub mod conferences;
pub use conferences::*;

pub mod paths;
pub use paths::*;

pub mod server_tab;
pub use server_tab::*;

use icy_board_tui::{config_menu::ResultState, TerminalType};
use ratatui::{layout::Rect, Frame};

pub trait TabPage {
    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn handle_key_press(&mut self, _key: KeyEvent) -> ResultState {
        ResultState::default()
    }

    fn request_edit_mode(&mut self, _terminal: &mut TerminalType, _full_screen: bool) -> ResultState {
        ResultState::default()
    }

    fn request_status(&self) -> ResultState {
        ResultState::default()
    }

    fn set_cursor_position(&self, _frame: &mut Frame) {}

    fn has_control(&self) -> bool {
        false
    }
}
