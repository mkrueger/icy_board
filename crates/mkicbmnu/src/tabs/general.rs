use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::icy_board::menu::{Menu, MenuType};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    hotkeys::{Hotkey, HotkeyBar},
    tab_page::TabPage,
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

const FIELDS: [&str; 7] = [
    "mnu_general_title",
    "mnu_general_display_file",
    "mnu_general_help_file",
    "mnu_general_menu_type",
    "mnu_general_prompt",
    "mnu_general_force_display",
    "mnu_general_pass_through",
];

pub struct GeneralTab {
    state: ConfigMenuState,
    config: ConfigMenu<Arc<Mutex<Menu>>>,
    original: Menu,
    observed: Menu,
    path: PathBuf,
}

fn menu_type_value(menu_type: &MenuType) -> ComboBoxValue {
    let key = match menu_type {
        MenuType::Hotkey => "mnu_general_type_hotkey",
        MenuType::Lightbar => "mnu_general_type_lightbar",
        MenuType::Command => "mnu_general_type_command",
    };
    ComboBoxValue::new(get_text(key), format!("{menu_type:?}"))
}

fn form(menu: Arc<Mutex<Menu>>, mnu: &Menu) -> ConfigMenu<Arc<Mutex<Menu>>> {
    let values = [
        ListValue::Text(u16::MAX, TextFlags::None, mnu.title.clone()),
        ListValue::Path(mnu.display_file.clone()),
        ListValue::Path(mnu.help_file.clone()),
        ListValue::ComboBox(ComboBox {
            cur_value: menu_type_value(&mnu.menu_type),
            selected_item: 0,
            is_edit_open: false,
            first_item: 0,
            values: MenuType::iter().map(|value| menu_type_value(&value)).collect(),
        }),
        ListValue::Text(u16::MAX, TextFlags::None, mnu.prompt.clone()),
        ListValue::Bool(mnu.force_display),
        ListValue::Bool(mnu.pass_through),
    ];
    ConfigMenu {
        obj: menu,
        // No render-time callbacks: publish before returning from key handling
        // so app-wide dirty tracking and undo see every edit immediately.
        entry: FIELDS
            .into_iter()
            .zip(values)
            .map(|(key, value)| {
                ConfigEntry::Item(
                    ListItem::new(get_text(key), value)
                        .with_status(get_text(&format!("{key}_status")))
                        .with_help(get_text(&format!("{key}_status"))),
                )
            })
            .collect(),
    }
}

fn same_fields(a: &Menu, b: &Menu) -> bool {
    a.title == b.title
        && a.display_file == b.display_file
        && a.help_file == b.help_file
        && a.menu_type == b.menu_type
        && a.prompt == b.prompt
        && a.force_display == b.force_display
        && a.pass_through == b.pass_through
}

impl GeneralTab {
    pub fn new(menu: Arc<Mutex<Menu>>, board_root: PathBuf, path: PathBuf) -> Self {
        let original = menu.lock().unwrap().clone();
        let mut state = ConfigMenuState::default();
        state.path_base = Some(board_root);
        Self {
            state,
            config: form(menu, &original),
            observed: original.clone(),
            original,
            path,
        }
    }

    /// The window title has no room next to the tabs, so the edited file is
    /// named here.
    fn frame_title(&self) -> String {
        match self.path.file_name() {
            Some(name) => get_text_args(
                "app_file_title",
                std::collections::HashMap::from([
                    ("application".to_string(), self.title()),
                    ("path".to_string(), name.to_string_lossy().into_owned()),
                ]),
            ),
            None => self.title(),
        }
    }

    /// Also called before rendering/input. Undo must replace the Menu inside
    /// the existing Arc. Unrelated edits and our own keystrokes retain cursor
    /// and popup state; external general-field changes reset the form.
    pub fn refresh_from_menu(&mut self) {
        let current = self.config.obj.lock().unwrap().clone();
        if !same_fields(&current, &self.observed) {
            self.config = form(self.config.obj.clone(), &current);
            let mut state = ConfigMenuState::default();
            state.path_base = self.state.path_base.clone();
            state.selected = self.state.selected.min(FIELDS.len() - 1);
            self.state = state;
        }
        self.observed = current;
    }

    fn publish_fields(&mut self) {
        let mut menu = self.config.obj.lock().unwrap();
        for index in 0..FIELDS.len() {
            match (index, &self.config.get_item(index).unwrap().value) {
                (0, ListValue::Text(_, _, value)) => menu.title.clone_from(value),
                (1, ListValue::Path(value)) => menu.display_file.clone_from(value),
                (2, ListValue::Path(value)) => menu.help_file.clone_from(value),
                (3, ListValue::ComboBox(value)) => {
                    if let Ok(value) = value.cur_value.value.parse() {
                        menu.menu_type = value;
                    }
                }
                (4, ListValue::Text(_, _, value)) => menu.prompt.clone_from(value),
                (5, ListValue::Bool(value)) => menu.force_display = *value,
                (6, ListValue::Bool(value)) => menu.pass_through = *value,
                _ => unreachable!(),
            }
        }
        self.observed = menu.clone();
    }

    fn render_type_popup(&self, frame: &mut Frame, area: Rect) {
        let ListValue::ComboBox(combo) = &self.config.get_item(3).unwrap().value else {
            return;
        };
        let width = (combo.values.iter().map(|value| Line::raw(&value.display).width()).max().unwrap_or(0) as u16 + 4)
            .max(36)
            .min(area.width);
        let height = (combo.values.len() as u16 + 2).min(area.height);
        let area = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);
        Clear.render(area, frame.buffer_mut());
        Block::new()
            .style(get_tui_theme().dialog_box)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title_bottom(
                HotkeyBar::new([
                    Hotkey::new(KeyCode::Enter, get_text("mnu_general_select")),
                    Hotkey::new(KeyCode::Esc, get_text("mnu_general_cancel")),
                ])
                .line(),
            )
            .render(area, frame.buffer_mut());
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let first = combo.selected_item.saturating_sub(inner.height.saturating_sub(1) as usize);
        for (row, value) in combo.values.iter().skip(first).take(inner.height as usize).enumerate() {
            Line::raw(&value.display)
                .style(if first + row == combo.selected_item {
                    get_tui_theme().selected_item
                } else {
                    get_tui_theme().value
                })
                .render(Rect::new(inner.x, inner.y + row as u16, inner.width, 1), frame.buffer_mut());
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        get_text("mnu_general_tab")
    }

    fn is_dirty(&self) -> bool {
        *self.config.obj.lock().unwrap() != self.original
    }

    fn has_control(&self) -> bool {
        self.state.is_path_browser_open()
            || self
                .config
                .get_item(self.state.selected)
                .is_some_and(|item| matches!(&item.value, ListValue::ComboBox(combo) if combo.is_edit_open))
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.refresh_from_menu();
        let width = area.width.min(78);
        let height = area.height.min(if self.has_control() { 22 } else { 13 });
        let area = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);
        Clear.render(area, frame.buffer_mut());
        let mut block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(get_tui_theme().dialog_box)
            .title(Line::raw(self.frame_title()).style(get_tui_theme().dialog_box_title));
        if !self.has_control() {
            let mut hints = vec![Hotkey::alternatives([KeyCode::Up, KeyCode::Down], get_text("mnu_general_move"))];
            match self.config.get_item(self.state.selected).map(|item| &item.value) {
                Some(ListValue::Path(_)) => hints.push(Hotkey::new(KeyCode::F(4), get_text("mnu_general_browse"))),
                Some(ListValue::Bool(_)) => hints.push(Hotkey::new(KeyCode::Char(' '), get_text("mnu_general_toggle"))),
                Some(ListValue::ComboBox(_)) => hints.push(Hotkey::new(KeyCode::Enter, get_text("mnu_general_select"))),
                _ => {}
            }
            hints.push(Hotkey::new(KeyCode::F(1), get_text("mnu_general_help")));
            block = block.title_bottom(HotkeyBar::new(hints).line());
        }
        block.render(area, frame.buffer_mut());
        let mut inner = area.inner(Margin { horizontal: 2, vertical: 1 });
        if !self.has_control() && inner.height >= 10 {
            Paragraph::new(get_text("mnu_general_runtime_note"))
                .style(get_tui_theme().menu_label)
                .wrap(Wrap { trim: true })
                .render(Rect::new(inner.x, inner.bottom() - 2, inner.width, 2), frame.buffer_mut());
            inner.height -= 3;
        }
        // ConfigMenu needs room for its separator, editor and scrollbar.
        if inner.width >= 12 && inner.height > 0 {
            let label_width = FIELDS.iter().map(|key| Line::raw(get_text(key)).width()).max().unwrap_or(0) as u16;
            let label_width = label_width.min(inner.width.saturating_sub(10));
            self.config.entry = std::mem::take(&mut self.config.entry)
                .into_iter()
                .map(|entry| match entry {
                    ConfigEntry::Item(item) => ConfigEntry::Item(item.with_label_width(label_width)),
                    other => other,
                })
                .collect();
            // ConfigMenu's stock combo popup sizes itself without clipping.
            // Render the closed value underneath our bounded overlay instead.
            let popup = if let ListValue::ComboBox(combo) = &mut self.config.get_item_mut(3).unwrap().value {
                let open = combo.is_edit_open;
                combo.is_edit_open = false;
                open
            } else {
                false
            };
            self.config.render(inner, frame, &mut self.state);
            if popup {
                if let ListValue::ComboBox(combo) = &mut self.config.get_item_mut(3).unwrap().value {
                    combo.is_edit_open = true;
                }
                self.render_type_popup(frame, area);
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        self.refresh_from_menu();
        if key.kind == KeyEventKind::Release {
            return self.request_status();
        }
        let result = self.config.handle_key_press(key, &mut self.state);
        self.publish_fields();
        result
    }

    fn request_status(&self) -> ResultState {
        ResultState::status_line(self.config.current_status_line(&self.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_tui::config_menu::EditMessage;
    use ratatui::{Terminal, backend::TestBackend};

    fn tab(menu: Arc<Mutex<Menu>>) -> GeneralTab {
        GeneralTab::new(menu, std::env::temp_dir(), PathBuf::from("/boards/demo/main.mnu"))
    }

    #[test]
    fn the_page_title_names_the_edited_file_because_the_window_title_cannot() {
        let mut tab = tab(Arc::new(Mutex::new(Menu::default())));
        assert!(tab.frame_title().contains(&tab.title()));
        assert!(tab.frame_title().contains("main.mnu"));
        assert!(!tab.frame_title().contains("/boards/demo"), "the directory would crowd the frame");
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        let rendered = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(rendered.contains("main.mnu"), "{rendered}");
    }

    #[test]
    fn localized_fields_and_statuses_render_at_80x25() {
        let mut tab = tab(Arc::new(Mutex::new(Menu::default())));
        assert_eq!(tab.title(), get_text("mnu_general_tab"));
        for (index, key) in FIELDS.iter().enumerate() {
            assert_eq!(tab.config.get_item(index).unwrap().status, get_text(&format!("{key}_status")));
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        let rendered = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect::<String>();
        for key in FIELDS {
            assert!(rendered.contains(&get_text(key)), "missing field: {key}");
        }
    }

    #[test]
    fn menu_types_keep_stable_values_and_escape_is_scoped() {
        let menu = Arc::new(Mutex::new(Menu::default()));
        let mut tab = tab(menu.clone());
        tab.state.selected = 3;
        for (index, menu_type) in MenuType::iter().enumerate() {
            tab.handle_key_press(KeyCode::Enter.into());
            assert!(tab.has_control());
            if let ListValue::ComboBox(combo) = &mut tab.config.get_item_mut(3).unwrap().value {
                assert_eq!(combo.values[index].value, format!("{menu_type:?}"));
                assert_eq!(combo.values[index].display, menu_type_value(&menu_type).display);
                combo.selected_item = index;
            }
            tab.handle_key_press(KeyCode::Enter.into());
            assert!(!tab.has_control());
            assert_eq!(menu.lock().unwrap().menu_type, menu_type);
        }
        tab.handle_key_press(KeyCode::Enter.into());
        assert!(tab.handle_key_press(KeyCode::Esc.into()).edit_msg == EditMessage::None);
        assert!(!tab.has_control());
    }

    #[test]
    fn edits_publish_immediately_preserve_other_fields_and_refresh_after_undo() {
        let mut value = Menu::default();
        value.prompts.push((".DEU".into(), "Sprache".into()));
        value.commands.push(Default::default());
        let menu = Arc::new(Mutex::new(value.clone()));
        let mut tab = tab(menu.clone());
        tab.handle_key_press(KeyCode::Char('A').into());
        tab.refresh_from_menu();
        tab.handle_key_press(KeyCode::Char('B').into());
        assert_eq!(menu.lock().unwrap().title, "AB");
        for index in [5, 6] {
            tab.state.selected = index;
            tab.handle_key_press(KeyCode::Char(' ').into());
        }
        let current = menu.lock().unwrap().clone();
        assert!(current.force_display && current.pass_through);
        assert!(current.commands == value.commands && current.prompts == value.prompts);
        assert!(tab.is_dirty());
        *menu.lock().unwrap() = value;
        tab.refresh_from_menu();
        assert!(matches!(&tab.config.get_item(0).unwrap().value, ListValue::Text(_, _, value) if value.is_empty()));
        assert!(matches!(&tab.config.get_item(5).unwrap().value, ListValue::Bool(false)));
        assert!(!tab.is_dirty());
    }

    #[test]
    fn unrelated_external_changes_do_not_reset_an_open_control() {
        let menu = Arc::new(Mutex::new(Menu::default()));
        let mut tab = tab(menu.clone());
        tab.state.selected = 3;
        tab.handle_key_press(KeyCode::Enter.into());
        menu.lock().unwrap().prompts.push((".SPA".into(), "Texto".into()));
        tab.refresh_from_menu();
        assert!(tab.has_control());
    }

    #[test]
    fn path_browser_uses_board_root_without_changing_cwd() {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "mkicbmnu-general-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("selected.txt"), "unchanged").unwrap();
        let cwd = std::env::current_dir().unwrap();
        let menu = Arc::new(Mutex::new(Menu::default()));
        let mut tab = GeneralTab::new(menu.clone(), root.clone(), PathBuf::from("main.mnu"));
        for index in [1, 2] {
            tab.state.selected = index;
            tab.handle_key_press(KeyCode::F(4).into());
            assert!(tab.has_control());
            tab.handle_key_press(KeyCode::End.into());
            tab.handle_key_press(KeyCode::Enter.into());
            assert!(!tab.has_control());
            assert!(matches!(&tab.config.get_item(index).unwrap().value, ListValue::Path(value) if value == &PathBuf::from("selected.txt")));
            tab.handle_key_press(KeyCode::F(4).into());
            assert!(tab.handle_key_press(KeyCode::Esc.into()).edit_msg == EditMessage::None);
            assert!(!tab.has_control());
        }
        assert_eq!(std::env::current_dir().unwrap(), cwd);
        assert_eq!(std::fs::read_to_string(root.join("selected.txt")).unwrap(), "unchanged");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rendering_is_bounded_and_keeps_long_text() {
        let mut menu = Menu::default();
        menu.title = "界 long title ".repeat(20);
        menu.prompt = "@X07 Long prompt ".repeat(30);
        let menu = Arc::new(Mutex::new(menu));
        let original = menu.lock().unwrap().clone();
        let mut tab = tab(menu.clone());
        for (width, height) in [(0, 0), (1, 1), (12, 4), (30, 8), (80, 25)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
            tab.state.selected = 3;
            tab.handle_key_press(KeyCode::Enter.into());
            terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
            tab.handle_key_press(KeyCode::Esc.into());
        }
        assert!(*menu.lock().unwrap() == original);
    }
}
