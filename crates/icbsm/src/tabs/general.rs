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
    tab_page::{Page, TabPage},
};
use ratatui::{Frame, layout::Rect};

use crate::VERSION;

use super::{
    ColorCustomization, DirColorEditor, EditorConfiguration, MaintenanceOp, MaintenancePage, MenuPage, UndoPage, UserList, security_menu_page,
    sort_options_page,
};

const MAIN_MENU: [(char, &str); 4] = [
    ('A', "icbsm_main_users"),
    ('B', "icbsm_main_directory"),
    ('C', "icbsm_define_editors"),
    ('D', "icbsm_customize_colors"),
];

const USERS_MENU: [(char, &str); 10] = [
    ('A', "icbsm_menu_edit_users"),
    ('B', "icbsm_menu_sort"),
    ('C', "icbsm_menu_pack"),
    ('D', "icbsm_menu_adjust_security"),
    ('E', "icbsm_menu_insert_conf"),
    ('F', "icbsm_menu_remove_conf"),
    ('G', "icbsm_menu_move_conf"),
    ('H', "icbsm_menu_expiration"),
    ('I', "icbsm_menu_phones"),
    ('J', "icbsm_menu_undo"),
];

pub struct GeneralTab {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl GeneralTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let center_title = get_text("icbsm_main_menu_title");
        let right_title = get_text_args("icb_setup_main_use_label", HashMap::from([("version".to_string(), VERSION.to_string())]));
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(
                MAIN_MENU
                    .iter()
                    .enumerate()
                    .map(|(id, (key, label))| MenuItem::new(id as i32, *key, get_text(label)))
                    .collect(),
            ))
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
                    let page = users_file_maintenance_page(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                1 => {
                    let page = directory_maintenance_page(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                2 => {
                    let page = EditorConfiguration::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                3 => {
                    let page = ColorCustomization::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                _ => return state,
            }
        }
        state
    }
}

fn users_file_maintenance_page(icy_board: Arc<Mutex<IcyBoard>>) -> MenuPage {
    MenuPage::new(
        get_text("icb_sysmanager_main_title"),
        USERS_MENU
            .iter()
            .enumerate()
            .map(|(id, (key, label))| MenuItem::new(id as i32, *key, get_text(label)))
            .collect(),
        Box::new(move |id| {
            let board = icy_board.clone();
            let page: Box<dyn Page> = match id {
                0 => Box::new(UserList::new(board)),
                1 => Box::new(sort_options_page(board)),
                2 => Box::new(MaintenancePage::new(board, MaintenanceOp::Pack)),
                3 => Box::new(security_menu_page(board)),
                4 => Box::new(MaintenancePage::new(board, MaintenanceOp::ConferenceInsert)),
                5 => Box::new(MaintenancePage::new(board, MaintenanceOp::ConferenceRemove)),
                6 => Box::new(MaintenancePage::new(board, MaintenanceOp::ConferenceMove)),
                7 => Box::new(MaintenancePage::new(board, MaintenanceOp::AdjustExpiration)),
                8 => Box::new(MaintenancePage::new(board, MaintenanceOp::StandardizePhones)),
                9 => Box::new(UndoPage::new(board)),
                _ => return None,
            };
            Some(page)
        }),
    )
}

fn directory_maintenance_page(icy_board: Arc<Mutex<IcyBoard>>) -> MenuPage {
    MenuPage::new(
        get_text("icbsm_main_directory"),
        vec![MenuItem::new(0, 'A', get_text("icbsm_dir_colors"))],
        Box::new(move |id| {
            if id == 0 {
                Some(Box::new(DirColorEditor::new(icy_board.clone())))
            } else {
                None
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::{MAIN_MENU, USERS_MENU};

    #[test]
    fn supported_menu_hierarchy_and_accelerators_are_preserved() {
        assert_eq!(MAIN_MENU.map(|(key, _)| key), ['A', 'B', 'C', 'D']);
        assert_eq!(USERS_MENU.map(|(key, _)| key), ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J']);
    }
}
