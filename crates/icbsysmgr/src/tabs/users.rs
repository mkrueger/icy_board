use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::config_menu::ConfigEntry;
use icy_board_tui::config_menu::ConfigMenu;
use icy_board_tui::config_menu::ConfigMenuState;
use icy_board_tui::config_menu::ListItem;
use icy_board_tui::config_menu::ListValue;
use icy_board_tui::tab_page::TabPage;
use icy_board_tui::{config_menu::ResultState, theme::get_tui_theme};
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::{
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
    Frame,
};

pub struct UsersTab {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,

    in_edit_mode: bool,

    conference_config: ConfigMenu<u32>,
    state: ConfigMenuState,
    edit_conference: usize,
}

impl UsersTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let items = vec![];

        Self {
            scroll_state: ScrollbarState::default().content_length(icy_board.lock().unwrap().conferences.len()),
            table_state: TableState::default().with_selected(0),
            icy_board: icy_board.clone(),
            in_edit_mode: false,
            conference_config: ConfigMenu { obj: 0, entry: items },
            state: ConfigMenuState::default(),
            edit_conference: 0,
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, mut area: Rect) {
        area.y += 1;
        area.height -= 1;
        frame.render_stateful_widget(
            Scrollbar::default()
                .style(get_tui_theme().content_box)
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .thumb_symbol("█")
                .track_symbol(Some("░"))
                .end_symbol(Some("▼")),
            area,
            &mut self.scroll_state,
        );
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["", "Name", "Alias", "Security"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(get_tui_theme().table_header)
            .height(1);

        let l = self.icy_board.lock().unwrap();
        let rows = l.users.iter().enumerate().map(|(i, user)| {
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)),
                Cell::from(user.name.clone()),
                Cell::from(user.alias.clone()),
                Cell::from(user.security_level.to_string()),
            ])
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(3 + 1),
            ],
        )
        .header(header)
        .row_highlight_style(get_tui_theme().selected_item)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
        //.bg(THEME.content.bg.unwrap())
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn prev(&mut self) {
        if self.icy_board.lock().unwrap().conferences.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.icy_board.lock().unwrap().users.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn next(&mut self) {
        if self.icy_board.lock().unwrap().conferences.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i + 1 >= self.icy_board.lock().unwrap().users.len() {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * 1);
    }

    fn insert(&mut self) {
        let mut conf = if let Some(i) = self.table_state.selected() {
            self.icy_board.lock().unwrap().conferences[i].clone()
        } else {
            self.icy_board.lock().unwrap().conferences[0].clone()
        };
        conf.name = format!("NewUser{}", self.icy_board.lock().unwrap().conferences.len() + 1);
        self.icy_board.lock().unwrap().conferences.push(conf);
        self.scroll_state = self.scroll_state.content_length(self.icy_board.lock().unwrap().conferences.len());
    }

    fn remove(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if i > 0 {
                self.icy_board.lock().unwrap().conferences.remove(i);
                let len = self.icy_board.lock().unwrap().conferences.len();
                self.scroll_state = self.scroll_state.content_length(len);

                if len >= i - 1 {
                    self.table_state.select(Some(i - 1));
                } else {
                    self.table_state.select(Some(0));
                }
            }
        }
    }

    fn render_editor(&mut self, frame: &mut Frame, area: Rect) {
        self.conference_config.render(area, frame, &mut self.state);
    }

    fn open_editor(&mut self, user_number: usize) {
        self.state = ConfigMenuState::default();
        self.edit_conference = user_number;
        let ib = self.icy_board.lock().unwrap();
        let user = ib.users.get(user_number).unwrap();

        let items = vec![
            ConfigEntry::Group(
                "Form".to_string(),
                vec![
                    ConfigEntry::Item(ListItem::new("Name".to_string(), ListValue::Text(25, user.name.clone())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Alias".to_string(), ListValue::Text(25, user.alias.clone())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("B/D Phone".to_string(), ListValue::Text(25, user.bus_data_phone.clone())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("H/V Phone".to_string(), ListValue::Text(25, user.home_voice_phone.clone())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Password".to_string(), ListValue::Text(25, user.password.password.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Security".to_string(), ListValue::U32(user.security_level as u32, 0, 255)).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Verify answer".to_string(), ListValue::Text(25, user.verify_answer.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("City/State".to_string(), ListValue::Text(25, user.city_or_state.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Expert".to_string(), ListValue::Bool(user.flags.expert_mode)).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Protocol".to_string(), ListValue::Text(2, user.protocol.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Page Len".to_string(), ListValue::U32(user.page_len as u32, 0, 255)).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Msg Clear".to_string(), ListValue::Bool(user.flags.msg_clear)).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Scroll Msgs".to_string(), ListValue::Bool(user.flags.scroll_msg_body)).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Short Desc".to_string(), ListValue::Bool(user.flags.use_short_filedescr)).with_label_width(14)),
                    ConfigEntry::Item(
                        ListItem::new("Last in".to_string(), ListValue::U32(user.last_conference as u32, 0, u16::MAX as u32)).with_label_width(14),
                    ),
                    ConfigEntry::Item(ListItem::new("Delete User".to_string(), ListValue::Bool(user.flags.delete_flag)).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Comment1".to_string(), ListValue::Text(60, user.user_comment.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Comment2".to_string(), ListValue::Text(60, user.sysop_comment.to_string())).with_label_width(14)),
                ],
            ),
            ConfigEntry::Group(
                "Address Form".to_string(),
                vec![
                    ConfigEntry::Item(ListItem::new("Address #1".to_string(), ListValue::Text(25, user.street1.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Address #2".to_string(), ListValue::Text(25, user.street2.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("City".to_string(), ListValue::Text(25, user.city.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("State".to_string(), ListValue::Text(25, user.state.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Zip Code".to_string(), ListValue::Text(25, user.zip.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Country".to_string(), ListValue::Text(25, user.country.to_string())).with_label_width(14)),
                ],
            ),
            ConfigEntry::Group(
                "Caller Notes".to_string(),
                vec![
                    ConfigEntry::Item(ListItem::new("Line 1".to_string(), ListValue::Text(60, user.custom_comment1.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Line 2".to_string(), ListValue::Text(60, user.custom_comment2.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Line 3".to_string(), ListValue::Text(60, user.custom_comment3.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Line 4".to_string(), ListValue::Text(60, user.custom_comment4.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Line 5".to_string(), ListValue::Text(60, user.custom_comment5.to_string())).with_label_width(14)),
                ],
            ),
            ConfigEntry::Group(
                "Personal".to_string(),
                vec![
                    ConfigEntry::Item(ListItem::new("Gender".to_string(), ListValue::Text(60, user.gender.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Birthdate".to_string(), ListValue::Text(60, user.birth_date.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Email Address".to_string(), ListValue::Text(60, user.email.to_string())).with_label_width(14)),
                    ConfigEntry::Item(ListItem::new("Web Address".to_string(), ListValue::Text(60, user.web.to_string())).with_label_width(14)),
                ],
            ),
        ];
        self.conference_config.entry = items;
    }
}

impl TabPage for UsersTab {
    fn title(&self) -> String {
        "Users".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });

        if self.in_edit_mode {
            self.render_editor(frame, area);
            self.set_cursor_position(frame);
            return;
        }

        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn set_cursor_position(&self, frame: &mut Frame) {
        self.conference_config
            .get_item(self.state.selected)
            .unwrap()
            .text_field_state
            .set_cursor_position(frame);
    }

    fn has_control(&self) -> bool {
        self.in_edit_mode
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if self.in_edit_mode {
            match key.code {
                KeyCode::Esc => {
                    self.in_edit_mode = false;
                    return ResultState::default();
                }
                _ => {
                    self.conference_config.handle_key_press(key, &mut self.state);
                    let home_dir = self.icy_board.lock().unwrap().config.paths.home_dir.clone();
                    if let Some(user) = self.icy_board.lock().unwrap().users.get_mut(self.edit_conference) {
                        let _ = user.save(&home_dir);
                    }
                }
            }
            return ResultState::status_line(String::new());
        }
        match key.code {
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('i') | KeyCode::Insert => self.insert(),
            KeyCode::Char('r') | KeyCode::Delete => self.remove(),
            KeyCode::Char('d') | KeyCode::Enter => {
                if let Some(state) = self.table_state.selected() {
                    self.in_edit_mode = true;
                    self.open_editor(state);
                    return ResultState::status_line(String::new());
                } else {
                    self.in_edit_mode = false;
                }
            }
            _ => {}
        }
        ResultState::default()
    }
}
