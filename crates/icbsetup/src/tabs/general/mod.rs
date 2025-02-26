use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use board_configuration::BoardConfiguration;
use configuration_options::ConfigurationOptions;
use connection_info::ConnectionInfo;
use crossterm::event::KeyEvent;
use event_setup::EventSetup;
use file_locations::FileLocations;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{EditMode, ResultState},
    get_text, get_text_args,
    icbsetupmenu::IcbSetupMenuUI,
    select_menu::{MenuItem, SelectMenu},
    tab_page::TabPage,
};
use ratatui::{Frame, layout::Rect};
use subscription_information::SubscriptionInformation;
mod accounting;
mod board_configuration;
pub mod conferences;
mod configuration_options;
mod connection_info;
mod event_setup;
mod file_locations;
mod new_user_options;
mod security_levels;
mod subscription_information;
mod sysop_information;

use crate::VERSION;

pub struct GeneralTab {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl GeneralTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let left_title = icy_board.as_ref().lock().unwrap().file_name.file_name().unwrap().to_string_lossy().to_string();
        let center_title = get_text("icb_setup_main_title");
        let right_title = get_text_args("icb_setup_main_use_label", HashMap::from([("version".to_string(), VERSION.to_string())]));
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("icb_setup_main_sysop_info")).with_help(get_text("icb_setup_main_sysop_info-help")),
                MenuItem::new(1, 'B', get_text("icb_setup_main_file_locs")).with_help(get_text("icb_setup_main_file_locs-help")),
                MenuItem::new(2, 'C', get_text("icb_setup_main_con_info")).with_help(get_text("icb_setup_main_con_info-help")),
                MenuItem::new(3, 'D', get_text("icb_setup_main_board_cfg")).with_help(get_text("icb_setup_main_board_cfg-help")),
                MenuItem::new(4, 'E', get_text("icb_setup_main_evt_setup")).with_help(get_text("icb_setup_main_evt_setup-help")),
                MenuItem::new(5, 'F', get_text("icb_setup_main_subscription")).with_help(get_text("icb_setup_main_subscription-help")),
                MenuItem::new(6, 'G', get_text("icb_setup_main_conf_opt")).with_help(get_text("icb_setup_main_conf_opt-help")),
                MenuItem::new(7, 'H', get_text("icb_setup_main_sec_levels")).with_help(get_text("icb_setup_main_sec_levels-help")),
                MenuItem::new(8, 'I', get_text("icb_setup_main_acc_cfg")).with_help(get_text("icb_setup_main_acc_cfg-help")),
                MenuItem::new(9, 'J', get_text("icb_setup_main_new_user")).with_help(get_text("icb_setup_main_new_user-help")),
                MenuItem::new(10, 'K', "TODO: Mailer/Tosser".to_string()),
                MenuItem::new(11, 'L', get_text("icb_setup_mb_conf")).with_help(get_text("icb_setup_mb_conf-help")),
                MenuItem::new(12, 'M', get_text("icb_setup_conferences")).with_help(get_text("icb_setup_conferences-help")),
            ]))
            .with_left_title(left_title)
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
        if matches!(state.edit_mode, EditMode::DisplayHelp(_)) {
            return state;
        }
        if let Some(selected) = opt {
            match selected {
                0 => {
                    let page = sysop_information::SysopInformation::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                1 => {
                    let page = FileLocations::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                2 => {
                    let page = ConnectionInfo::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                3 => {
                    let page = BoardConfiguration::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                4 => {
                    let page = EventSetup::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                5 => {
                    let page = SubscriptionInformation::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                6 => {
                    let page = ConfigurationOptions::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                7 => {
                    let page = security_levels::SecurityLevelOptions::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                8 => {
                    let page = accounting::AccountingConfig::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                9 => {
                    let page = new_user_options::NewUserOptions::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                11 => {
                    // Main Board Configuration
                    let page = conferences::ConferenceEditor::new(self.icy_board.clone(), 0);
                    return self.page.open_sup_page(Box::new(page));
                }
                12 => {
                    let page = conferences::ConferenceListEditor::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                _ => {}
            }
        }
        state
    }
}
