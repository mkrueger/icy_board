use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{EditMessage, ResultState},
    get_text, get_text_args,
    icbsetupmenu::IcbSetupMenuUI,
    select_menu::{MenuItem, SelectMenu},
    tab_page::TabPage,
};
use ratatui::{Frame, layout::Rect};

use crate::VERSION;

use super::{GroupEditor, UserList};

pub struct GeneralTab {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl GeneralTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let center_title = get_text("icb_sysmanager_main_title");
        let right_title = get_text_args("icb_setup_main_use_label", HashMap::from([("version".to_string(), VERSION.to_string())]));
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("icb_sysmanager_main_edit_users")),
                MenuItem::new(1, 'B', get_text("icb_sysmanager_main_edit_groups")),
            ]))
            .with_center_title(center_title)
            .with_right_title(right_title),
            icy_board,
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        "Main".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn has_control(&self) -> bool {
        !self.page.sub_pages.is_empty()
    }

    fn request_status(&self) -> ResultState {
        self.page.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        let (state, opt) = self.page.handle_key_press(key);
        if matches!(state.edit_msg, EditMessage::DisplayHelp(_)) {
            return state;
        }
        if let Some(selected) = opt {
            match selected {
                0 => {
                    let page = UserList::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                1 => {
                    let page = GroupEditor::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                _ => {}
            }
        }
        state
    }
}
