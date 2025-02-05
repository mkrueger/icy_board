use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

pub struct ConferenceEditor {
    pub state: ConfigMenuState,

    num_conf: usize,
    menu: ConfigMenu<u32>,
}

impl ConferenceEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, num_conf: usize) -> Self {
        let menu = {
            let ib = icy_board.lock().unwrap();
            let conf = ib.conferences.get(num_conf).unwrap();
            let entry = vec![
                ConfigEntry::Item(ListItem::new(format!("Name (#{})", num_conf + 1), ListValue::Text(25, conf.name.clone())).with_label_width(14)),
                ConfigEntry::Table(
                    2,
                    vec![
                        ConfigEntry::Item(ListItem::new("Public Conference".to_string(), ListValue::Bool(conf.is_public))),
                        ConfigEntry::Item(
                            ListItem::new("Req. Security if Public".to_string(), ListValue::Text(50, conf.required_security.to_string())).with_label_width(28),
                        ),
                    ],
                ),
                ConfigEntry::Item(
                    ListItem::new("Password to Join if Private".to_string(), ListValue::Text(25, conf.password.to_string())).with_label_width(28),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Item(ListItem::new("Name/Loc of User's Menu".to_string(), ListValue::Path(conf.users_menu.clone())).with_label_width(28)),
                ConfigEntry::Item(ListItem::new("Name/Loc of Sysop's Menu".to_string(), ListValue::Path(conf.sysop_menu.clone())).with_label_width(28)),
                ConfigEntry::Item(ListItem::new("Name/Loc of NEWS File".to_string(), ListValue::Path(conf.news_file.clone())).with_label_width(28)),
                ConfigEntry::Item(ListItem::new("Name/Loc of Conf INTRO File".to_string(), ListValue::Path(conf.intro_file.clone())).with_label_width(28)),
                ConfigEntry::Item(
                    ListItem::new("Location for Attachments".to_string(), ListValue::Path(conf.attachment_location.clone())).with_label_width(28),
                ),
                ConfigEntry::Item(ListItem::new("Conf. CMD.LST File".to_string(), ListValue::Path(conf.command_file.clone())).with_label_width(28)),
                ConfigEntry::Separator,
                ConfigEntry::Item(ListItem::new("Public Up Path".to_string(), ListValue::Path(conf.pub_upload_location.clone())).with_label_width(1)),
                ConfigEntry::Item(ListItem::new("Private Up Path".to_string(), ListValue::Path(conf.private_upload_location.clone())).with_label_width(1)),
                ConfigEntry::Separator,
                ConfigEntry::Table(
                    2,
                    vec![
                        ConfigEntry::Item(ListItem::new("Doors".to_string(), ListValue::Path(conf.doors_menu.clone())).with_label_width(12)),
                        ConfigEntry::Item(ListItem::new("".to_string(), ListValue::Path(conf.doors_file.clone()))),
                        ConfigEntry::Item(ListItem::new("Bulletins".to_string(), ListValue::Path(conf.blt_menu.clone())).with_label_width(12)),
                        ConfigEntry::Item(ListItem::new("".to_string(), ListValue::Path(conf.blt_file.clone()))),
                        ConfigEntry::Item(ListItem::new("Surveys".to_string(), ListValue::Path(conf.survey_menu.clone())).with_label_width(12)),
                        ConfigEntry::Item(ListItem::new("".to_string(), ListValue::Path(conf.survey_file.clone()))),
                        ConfigEntry::Item(ListItem::new("Directories".to_string(), ListValue::Path(conf.dir_menu.clone())).with_label_width(12)),
                        ConfigEntry::Item(ListItem::new("".to_string(), ListValue::Path(conf.dir_file.clone()))),
                        ConfigEntry::Item(ListItem::new("Areas".to_string(), ListValue::Path(conf.area_menu.clone())).with_label_width(12)),
                        ConfigEntry::Item(ListItem::new("".to_string(), ListValue::Path(conf.area_file.clone()))),
                    ],
                ),
            ];
            ConfigMenu { obj: 0, entry }
        };

        Self {
            state: ConfigMenuState::default(),
            num_conf,
            menu,
        }
    }
}

impl Page for ConferenceEditor {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = format!("Conference {}", self.num_conf).to_string();
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().menu_title).render(
            Rect {
                x: area.x + 1 + area.width.saturating_sub(width) / 2,
                y: area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );

        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().content_box,
        );

        let area = area.inner(Margin { vertical: 4, horizontal: 1 });
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        let res = self.menu.handle_key_press(key, &mut self.state);

        PageMessage::ResultState(res)
    }
}
