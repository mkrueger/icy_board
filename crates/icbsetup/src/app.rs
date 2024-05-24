use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    app::{App, Mode},
    help_view::HelpViewState,
};

use crate::{
    editors::{
        bullettins::BullettinsEditor, dirs::DirsEditor, door::DoorEditor, messages::MessageAreasEditor, sec_editor::SecurityLevelEditor, surveys::SurveyEditor,
    },
    tabs::{AboutTab, ConferencesTab, GeneralTab, PathTab, ServerTab},
};

pub fn new_main_window<'a>(icy_board: Arc<Mutex<IcyBoard>>, full_screen: bool) -> App<'a> {
    let general_tab = GeneralTab::new(icy_board.clone());
    let server_tab = ServerTab::new(icy_board.clone());
    let command_tab = ConferencesTab::new(icy_board.clone());
    let path_tab = PathTab::new(icy_board.clone());
    let date_format = icy_board.lock().unwrap().config.board.date_format.clone();
    App {
        full_screen,
        title: " IcyBoard Setup Utility".to_string(),
        mode: Mode::default(),
        tab: 0,
        date_format,
        tabs: vec![
            Box::new(general_tab),
            Box::new(path_tab),
            Box::new(server_tab),
            Box::new(command_tab),
            Box::new(AboutTab::default()),
        ],
        status_line: String::new(),
        help_state: HelpViewState::new(23),
        open_editor: None,
        get_editor: Box::new(|id, path| match id {
            "doors_file" => {
                return Ok(Some(Box::new(DoorEditor::new(path).unwrap())));
            }
            "pwrd_sec_level_file" => {
                return Ok(Some(Box::new(SecurityLevelEditor::new(path).unwrap())));
            }
            "blt_file" => {
                return Ok(Some(Box::new(BullettinsEditor::new(path).unwrap())));
            }
            "survey_file" => {
                return Ok(Some(Box::new(SurveyEditor::new(path).unwrap())));
            }

            "dir_file" => {
                return Ok(Some(Box::new(DirsEditor::new(path).unwrap())));
            }
            "area_file" => {
                return Ok(Some(Box::new(MessageAreasEditor::new(path).unwrap())));
            }

            _ => {
                panic!("Unknown id: {}", id);
            }
        }),
        save: false,
    }
}
