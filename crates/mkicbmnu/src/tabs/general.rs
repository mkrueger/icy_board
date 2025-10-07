use core::panic;
use std::{
    sync::{Arc, Mutex},
    vec,
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::menu::{Menu, MenuType};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListItem, ListValue, ResultState, TextFlags},
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

impl GeneralTab {
    pub fn new(menu: Arc<Mutex<Menu>>) -> Self {
        let info_width = 16;
        let original = menu.lock().unwrap().clone();
        let items = if let Ok(mnu) = menu.lock() {
            vec![
                ConfigEntry::Item(
                    ListItem::new("Title".to_string(), ListValue::Text(25, TextFlags::None, mnu.title.clone()))
                        .with_status("Enter the title of the menu.")
                        .with_label_width(info_width)
                        .with_update_text_value(&|mnu: &Arc<Mutex<Menu>>, value: String| {
                            mnu.lock().unwrap().title = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new("Display File".to_string(), ListValue::Path(mnu.display_file.clone()))
                        .with_status("The menu background file to display.")
                        .with_label_width(info_width)
                        .with_update_path_value(&|mnu: &Arc<Mutex<Menu>>, value: std::path::PathBuf| {
                            mnu.lock().unwrap().display_file = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new("Help File".to_string(), ListValue::Path(mnu.help_file.clone()))
                        .with_status("The help file to display.")
                        .with_label_width(info_width)
                        .with_update_path_value(&|mnu: &Arc<Mutex<Menu>>, value: std::path::PathBuf| {
                            mnu.lock().unwrap().help_file = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        "Menu Type".to_string(),
                        ListValue::ComboBox(ComboBox {
                            cur_value: ComboBoxValue::new(format!("{:?}", mnu.menu_type), format!("{:?}", mnu.menu_type)),
                            selected_item: 0,
                            is_edit_open: false,
                            first_item: 0,
                            values: MenuType::iter()
                                .map(|x| ComboBoxValue::new(format!("{:?}", x), format!("{:?}", x)))
                                .collect::<Vec<ComboBoxValue>>(),
                        }),
                    )
                    .with_status("The type of the menu.")
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
                    ListItem::new("Prompt".to_string(), ListValue::Text(25, TextFlags::None, mnu.prompt.clone()))
                        .with_status("The prompt for the menu.")
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
        "General".to_string()
    }
    fn is_dirty(&self) -> bool {
        self.config.obj.lock().unwrap().clone() != self.original
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let width = (2 + 50 + 2).min(area.width) as u16;

        let lines = (7).min(area.height) as u16;
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
        let res = self.config.handle_key_press(key, &mut self.state);
        res
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            edit_msg: EditMessage::None,
            status_line: if self.state.selected < self.config.entry.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }
}
