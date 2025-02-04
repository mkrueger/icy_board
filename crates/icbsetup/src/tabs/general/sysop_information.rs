use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{user_base::Password, IcyBoard};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::Margin,
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

pub struct SysopInformation {
    pub state: ConfigMenuState,

    icy_board: Arc<Mutex<IcyBoard>>,
    menu: ConfigMenu,
}

impl SysopInformation {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let sysop_info_width = 30;
            let sysop_info = vec![
                ConfigEntry::Item(
                    ListItem::new("sysop_name", "Sysop's Name".to_string(), ListValue::Text(45, lock.config.sysop.name.clone()))
                        .with_status("Enter the first name of the sysop.")
                        .with_label_width(sysop_info_width),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        "local_pass",
                        "Local Password".to_string(),
                        ListValue::Text(25, lock.config.sysop.password.to_string().clone()),
                    )
                    .with_status("Call waiting screen password.")
                    .with_label_width(sysop_info_width),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        "local_pass_exit",
                        "Require Password to Exit".to_string(),
                        ListValue::Bool(lock.config.sysop.require_password_to_exit),
                    )
                    .with_label_width(sysop_info_width)
                    .with_status("IcyBoard requires pw to exit the call waiting screen."),
                ),
                ConfigEntry::Item(
                    ListItem::new("use_real_name", "Use Real Name".to_string(), ListValue::Bool(lock.config.sysop.use_real_name))
                        .with_label_width(sysop_info_width)
                        .with_status("Message to sysop with real name?"),
                ),
            ];
            ConfigMenu { entry: sysop_info }
        };

        Self {
            state: ConfigMenuState::default(),
            icy_board,
            menu,
        }
    }

    fn write_item(&self, item: &ListItem, icy_board: &mut IcyBoard) {
        match &item.value {
            ListValue::Text(_, text) => match item.id.as_str() {
                "sysop_name" => icy_board.config.sysop.name = text.clone(),
                "local_pass" => icy_board.config.sysop.password = Password::PlainText(text.clone()),
                _ => {}
            },
            ListValue::Bool(value) => match item.id.as_str() {
                "local_pass_exit" => icy_board.config.sysop.require_password_to_exit = *value,
                "use_real_name" => icy_board.config.sysop.use_real_name = *value,
                _ => {}
            },
            _ => {}
        }
    }
}

impl Page for SysopInformation {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box)
            .title_top(
                Line::from(" Sysop Information ".to_string())
                    .style(get_tui_theme().content_box_title)
                    .centered(),
            );
        block.render(area, frame.buffer_mut());

        let area = area.inner(Margin { vertical: 3, horizontal: 1 });
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == crossterm::event::KeyCode::Esc {
            return PageMessage::Close;
        }
        let res = self.menu.handle_key_press(key, &mut self.state);
        for item in self.menu.iter() {
            self.write_item(item, &mut self.icy_board.lock().unwrap());
        }
        PageMessage::ResultState(res)
    }
}
