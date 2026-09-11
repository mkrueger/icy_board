use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{
    IcyBoard,
    doors::DropFile,
    icb_config::{ExternalEditorConfig, ExternalEditorMode},
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct ExternalEditor {
    menu: ICBConfigMenuUI,
}

fn item(key: &str, value: ListValue) -> ListItem<Arc<Mutex<IcyBoard>>> {
    ListItem::new(get_text(key), value)
        .with_label_width(29)
        .with_edit_width(38)
        .with_status(get_text(&format!("{key}-status")))
        .with_help(get_text(&format!("{key}-help")))
}

fn number(key: &str, value: u32, maximum: u32, update: fn(&mut ExternalEditorConfig, u32)) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(item(key, ListValue::U32(value, 1, maximum)).with_update_value(Box::new(move |board, value| {
        if let ListValue::U32(value, _, _) = value {
            update(&mut board.lock().unwrap().config.message.external_editor, *value);
        }
    })))
}

impl ExternalEditor {
    pub fn new(board: Arc<Mutex<IcyBoard>>) -> Self {
        let config = board.lock().unwrap().config.message.external_editor.clone();
        let modes = [
            (ExternalEditorMode::Internal, "internal"),
            (ExternalEditorMode::Program, "program"),
            (ExternalEditorMode::Script, "script"),
            (ExternalEditorMode::Dos, "dos"),
            (ExternalEditorMode::Ppe, "ppe"),
        ];
        let mode_values: Vec<_> = modes
            .iter()
            .map(|(mode, key)| ComboBoxValue::new(get_text(&format!("external_editor_mode_{key}")), mode.to_string()))
            .collect();
        let current = mode_values.iter().find(|value| value.value == config.mode.to_string()).unwrap().clone();
        let entry = vec![
            ConfigEntry::Separator,
            ConfigEntry::Item(
                item(
                    "external_editor_mode",
                    ListValue::ComboBox(ComboBox {
                        cur_value: current,
                        selected_item: 0,
                        is_edit_open: false,
                        first_item: 0,
                        values: mode_values,
                    }),
                )
                .with_update_combobox_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                    if let Ok(mode) = value.cur_value.value.parse() {
                        board.lock().unwrap().config.message.external_editor.mode = mode;
                    }
                }),
            ),
            ConfigEntry::Item(
                item("external_editor_path", ListValue::Text(255, TextFlags::None, config.path)).with_update_text_value(
                    &|board: &Arc<Mutex<IcyBoard>>, value| {
                        board.lock().unwrap().config.message.external_editor.path = value;
                    },
                ),
            ),
            ConfigEntry::Item(
                item("external_editor_arguments", ListValue::Text(1024, TextFlags::None, config.arguments)).with_update_text_value(
                    &|board: &Arc<Mutex<IcyBoard>>, value| {
                        board.lock().unwrap().config.message.external_editor.arguments = value;
                    },
                ),
            ),
            ConfigEntry::Item(
                item(
                    "external_editor_dropfile",
                    ListValue::ComboBox(ComboBox {
                        cur_value: ComboBoxValue::new(config.drop_file.to_string(), format!("{:?}", config.drop_file)),
                        selected_item: 0,
                        is_edit_open: false,
                        first_item: 0,
                        values: DropFile::iter()
                            .map(|value| ComboBoxValue::new(value.to_string(), format!("{value:?}")))
                            .collect(),
                    }),
                )
                .with_update_combobox_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                    if let Some(drop_file) = DropFile::iter().find(|entry| format!("{entry:?}") == value.cur_value.value) {
                        board.lock().unwrap().config.message.external_editor.drop_file = drop_file;
                    }
                }),
            ),
            number("external_editor_timeout", config.timeout_seconds, 86400, |config, value| {
                config.timeout_seconds = value
            }),
            number("external_editor_memory", config.dos_memory_mb, 512, |config, value| {
                config.dos_memory_mb = value
            }),
        ];
        Self {
            menu: ICBConfigMenuUI::new(get_text("external_editor_title"), ConfigMenu { obj: board, entry }),
        }
    }
}

impl Page for ExternalEditor {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        self.menu.render(frame, area);
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn external_editor_form_renders_and_edits_at_80_columns() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let mut page = ExternalEditor::new(board.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| page.render(frame, frame.area())).unwrap();
        let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
        for key in [
            "external_editor_title",
            "external_editor_mode",
            "external_editor_path",
            "external_editor_arguments",
            "external_editor_dropfile",
            "external_editor_timeout",
            "external_editor_memory",
        ] {
            assert!(text.contains(&get_text(key)), "missing label: {key}");
        }
        page.handle_key_press(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        page.handle_key_press(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        terminal.draw(|frame| page.render(frame, frame.area())).unwrap();
        assert_eq!(board.lock().unwrap().config.message.external_editor.path, "x");
        page.handle_key_press(KeyCode::Up.into());
        page.handle_key_press(KeyCode::Enter.into());
        for _ in 0..4 {
            page.handle_key_press(KeyCode::Down.into());
        }
        page.handle_key_press(KeyCode::Enter.into());
        terminal.draw(|frame| page.render(frame, frame.area())).unwrap();
        let config = board.lock().unwrap().config.message.external_editor.clone();
        assert_eq!(config.mode, ExternalEditorMode::Ppe);
        let stored = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<ExternalEditorConfig>(&stored).unwrap(), config);
        assert_eq!(toml::from_str::<ExternalEditorConfig>("").unwrap(), ExternalEditorConfig::default());
    }
}
