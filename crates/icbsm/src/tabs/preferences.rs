use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::{IcbColor, PCB_SCREEN_COLOR_NAMES, PcbScreenColors},
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    icbsetupmenu::IcbSetupMenuUI,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
    theme::set_tui_theme,
};
use ratatui::{Frame, layout::Rect};

pub struct EditorConfiguration {
    menu: ICBConfigMenuUI,
}

impl EditorConfiguration {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let lock = icy_board.lock().unwrap();
        let entries = vec![
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    get_text("icbsm_text_editor"),
                    ListValue::Text(128, TextFlags::None, lock.config.sysop.external_editor.clone()),
                )
                .with_label_width(20)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                    board.lock().unwrap().config.sysop.external_editor = value;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("icbsm_graphics_editor"),
                    ListValue::Text(128, TextFlags::None, lock.config.sysop.graphics_editor.clone()),
                )
                .with_label_width(20)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                    board.lock().unwrap().config.sysop.graphics_editor = value;
                }),
            ),
        ];
        drop(lock);
        Self {
            menu: ICBConfigMenuUI::new(
                get_text("icbsm_define_editors"),
                ConfigMenu {
                    obj: icy_board,
                    entry: entries,
                },
            ),
        }
    }
}

impl Page for EditorConfiguration {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.menu.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}

pub struct ColorCustomization {
    page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl ColorCustomization {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("icbsm_color_default_1")),
                MenuItem::new(1, 'B', get_text("icbsm_color_default_2")),
                MenuItem::new(2, 'C', get_text("icbsm_color_bw")),
                MenuItem::new(3, 'D', get_text("icbsm_color_customize")),
            ]))
            .with_center_title(get_text("icbsm_color_title")),
            icy_board,
        }
    }

    fn apply_preset(&self, name: &str, palette: PcbScreenColors) {
        let mut board = self.icy_board.lock().unwrap();
        board.config.sysop.config_color_theme = name.to_string();
        board.config.sysop.config_color_configuration = palette.clone();
        set_tui_theme(&palette);
    }
}

impl Page for ColorCustomization {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.page.sub_pages.is_empty() && key.code == KeyCode::Esc {
            return PageMessage::Close;
        }
        let (state, selected) = self.page.handle_key_press(key);
        match selected {
            Some(0) => self.apply_preset("DEFAULT1", PcbScreenColors::default()),
            Some(1) => self.apply_preset("DEFAULT2", PcbScreenColors::default_2()),
            Some(2) => self.apply_preset("BLACK_AND_WHITE", PcbScreenColors::black_and_white()),
            Some(3) => return PageMessage::OpenSubPage(Box::new(CustomColorEditor::new(self.icy_board.clone()))),
            _ => {}
        }
        PageMessage::ResultState(state)
    }
}

struct CustomColorEditor {
    menu: ICBConfigMenuUI,
}

impl CustomColorEditor {
    fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let colors = icy_board.lock().unwrap().config.sysop.config_color_configuration.colors;
        let mut entries = vec![ConfigEntry::Separator];
        for (index, name) in PCB_SCREEN_COLOR_NAMES.iter().enumerate() {
            entries.push(ConfigEntry::Item(
                ListItem::new(name.to_string(), ListValue::Color(IcbColor::Dos(colors[index])))
                    .with_label_width(34)
                    .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value| {
                        let ListValue::Color(IcbColor::Dos(color)) = value else {
                            return;
                        };
                        let mut board = board.lock().unwrap();
                        board.config.sysop.config_color_theme = "CUSTOM".to_string();
                        board.config.sysop.config_color_configuration.colors[index] = *color;
                        set_tui_theme(&board.config.sysop.config_color_configuration);
                    })),
            ));
        }
        Self {
            menu: ICBConfigMenuUI::new(
                get_text("icbsm_color_customize"),
                ConfigMenu {
                    obj: icy_board,
                    entry: entries,
                },
            ),
        }
    }
}

impl Page for CustomColorEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.menu.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}
