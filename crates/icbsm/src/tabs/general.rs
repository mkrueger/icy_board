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

use super::{ColorCustomization, EditorConfiguration, GroupEditor, MaintenanceOp, MaintenancePage, UndoPage, UserList, security_menu_page, sort_options_page};

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
                MenuItem::new(0, 'A', get_text("icbsm_menu_edit_users")),
                MenuItem::new(1, 'B', get_text("icbsm_menu_sort")),
                MenuItem::new(2, 'C', get_text("icbsm_menu_pack")),
                MenuItem::new(3, 'D', get_text("icbsm_menu_adjust_security")),
                MenuItem::new(4, 'E', get_text("icbsm_menu_insert_conf")),
                MenuItem::new(5, 'F', get_text("icbsm_menu_remove_conf")),
                MenuItem::new(6, 'G', get_text("icbsm_menu_move_conf")),
                MenuItem::new(7, 'H', get_text("icbsm_menu_expiration")),
                MenuItem::new(8, 'I', get_text("icbsm_menu_phones")),
                MenuItem::new(9, 'J', get_text("icbsm_menu_undo")),
                MenuItem::new(10, 'K', get_text("icbsm_menu_groups")),
                MenuItem::new(11, 'L', get_text("icbsm_define_editors")),
                MenuItem::new(12, 'M', get_text("icbsm_customize_colors")),
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
            let op = match selected {
                0 => {
                    let page = UserList::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                1 => {
                    let page = sort_options_page(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                2 => MaintenanceOp::Pack,
                3 => {
                    let page = security_menu_page(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                4 => MaintenanceOp::ConferenceInsert,
                5 => MaintenanceOp::ConferenceRemove,
                6 => MaintenanceOp::ConferenceMove,
                7 => MaintenanceOp::AdjustExpiration,
                8 => MaintenanceOp::StandardizePhones,
                9 => {
                    let page = UndoPage::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                10 => {
                    let page = GroupEditor::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                11 => {
                    let page = EditorConfiguration::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                12 => {
                    let page = ColorCustomization::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                _ => return state,
            };
            let page = MaintenancePage::new(self.icy_board.clone(), op);
            return self.page.open_sup_page(Box::new(page));
        }
        state
    }
}
