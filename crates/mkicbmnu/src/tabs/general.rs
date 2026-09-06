use core::panic;
use std::{
    sync::{Arc, Mutex},
    vec,
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::menu::{Menu, MenuType};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    tab_page::TabPage,
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
};

pub struct GeneralTab {
    state: ConfigMenuState,
    config: ConfigMenu<Arc<Mutex<Menu>>>,
    original: Menu,
}

fn menu_type_value(menu_type: &MenuType) -> ComboBoxValue {
    let key = match menu_type {
        MenuType::Hotkey => "mnu_editor_type_hotkey",
        MenuType::Lightbar => "mnu_editor_type_lightbar",
        MenuType::Command => "mnu_editor_type_command",
    };
    ComboBoxValue::new(get_text(key), format!("{menu_type:?}"))
}

impl GeneralTab {
    pub fn new(menu: Arc<Mutex<Menu>>) -> Self {
        let info_width = 16;
        let original = menu.lock().unwrap().clone();
        let items = if let Ok(mnu) = menu.lock() {
            vec![
                ConfigEntry::Item(
                    ListItem::new(get_text("mnu_editor_title"), ListValue::Text(25, TextFlags::None, mnu.title.clone()))
                        .with_status(get_text("mnu_editor_title_status"))
                        .with_label_width(info_width)
                        .with_update_text_value(&|mnu: &Arc<Mutex<Menu>>, value: String| {
                            mnu.lock().unwrap().title = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("mnu_editor_display_file"), ListValue::Path(mnu.display_file.clone()))
                        .with_status(get_text("mnu_editor_display_file_status"))
                        .with_label_width(info_width)
                        .with_update_path_value(&|mnu: &Arc<Mutex<Menu>>, value: std::path::PathBuf| {
                            mnu.lock().unwrap().display_file = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("mnu_editor_help_file"), ListValue::Path(mnu.help_file.clone()))
                        .with_status(get_text("mnu_editor_help_file_status"))
                        .with_label_width(info_width)
                        .with_update_path_value(&|mnu: &Arc<Mutex<Menu>>, value: std::path::PathBuf| {
                            mnu.lock().unwrap().help_file = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("mnu_editor_menu_type"),
                        ListValue::ComboBox(ComboBox {
                            cur_value: menu_type_value(&mnu.menu_type),
                            selected_item: 0,
                            is_edit_open: false,
                            first_item: 0,
                            values: MenuType::iter().map(|x| menu_type_value(&x)).collect::<Vec<ComboBoxValue>>(),
                        }),
                    )
                    .with_status(get_text("mnu_editor_menu_type_status"))
                    .with_label_width(info_width)
                    .with_update_combobox_value(&|mnu: &Arc<Mutex<Menu>>, value: &ComboBox| {
                        let menu_type = match value.cur_value.value.as_str() {
                            "Hotkey" => MenuType::Hotkey,
                            "Lightbar" => MenuType::Lightbar,
                            _ => MenuType::Command,
                        };
                        mnu.lock().unwrap().menu_type = menu_type;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("mnu_editor_prompt"), ListValue::Text(25, TextFlags::None, mnu.prompt.clone()))
                        .with_status(get_text("mnu_editor_prompt_status"))
                        .with_label_width(info_width)
                        .with_update_text_value(&|mnu: &Arc<Mutex<Menu>>, value: String| {
                            mnu.lock().unwrap().prompt = value;
                        }),
                ),
            ]
        } else {
            panic!();
        };
        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                obj: menu,
                entry: vec![ConfigEntry::Group(String::new(), items)],
            },
            original,
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        get_text("tui_tab_general")
    }
    fn is_dirty(&self) -> bool {
        self.config.obj.lock().unwrap().clone() != self.original
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let width = (2 + 50 + 2).min(area.width);

        let lines = (7).min(area.height);
        let area = Rect::new(area.x + (area.width - width) / 2, (area.y + area.height - lines) / 2, width + 2, lines);

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame, &mut self.state);
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        self.config.handle_key_press(key, &mut self.state)
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: EditMessage::None,
            status_line: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn general_tab_uses_localized_title_fields_and_statuses() {
        let mut tab = GeneralTab::new(Arc::new(Mutex::new(Menu::default())));
        assert_eq!(tab.title(), get_text("tui_tab_general"));
        let keys = [
            "mnu_editor_title",
            "mnu_editor_display_file",
            "mnu_editor_help_file",
            "mnu_editor_menu_type",
            "mnu_editor_prompt",
        ];
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(tab.config.get_item(i).unwrap().status, get_text(&format!("{key}_status")));
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        let rendered = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect::<String>();
        for key in keys {
            let text = get_text(key);
            assert_ne!(text, key, "missing translation: {key}");
            assert!(rendered.contains(&text), "missing field: {key}");
        }
    }

    #[test]
    fn menu_type_choices_keep_stable_values() {
        let menu = Arc::new(Mutex::new(Menu::default()));
        let tab = GeneralTab::new(menu.clone());
        let item = tab.config.get_item(3).unwrap();
        let ListValue::ComboBox(combo) = &item.value else {
            core::panic!("expected menu type combo box");
        };
        for ((menu_type, key), value) in MenuType::iter()
            .zip(["mnu_editor_type_hotkey", "mnu_editor_type_lightbar", "mnu_editor_type_command"])
            .zip(&combo.values)
        {
            assert_eq!(value.value, format!("{menu_type:?}"));
            assert_eq!(value.display, get_text(key));
            let selection = ListValue::ComboBox(ComboBox {
                cur_value: value.clone(),
                values: combo.values.clone(),
                selected_item: 0,
                first_item: 0,
                is_edit_open: false,
            });
            item.update_value.as_ref().unwrap()(&menu, &selection);
            assert_eq!(menu.lock().unwrap().menu_type, menu_type);
        }
    }
}
