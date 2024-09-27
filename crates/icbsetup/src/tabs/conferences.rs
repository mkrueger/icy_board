use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::security_expr::SecurityExpression;
use icy_board_engine::icy_board::user_base::Password;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::config_menu::ConfigEntry;
use icy_board_tui::config_menu::ConfigMenu;
use icy_board_tui::config_menu::ConfigMenuState;
use icy_board_tui::config_menu::EditMode;
use icy_board_tui::config_menu::ListItem;
use icy_board_tui::config_menu::ListValue;
use icy_board_tui::tab_page::TabPage;
use icy_board_tui::{config_menu::ResultState, theme::THEME};
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

pub struct ConferencesTab {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,

    in_edit_mode: bool,

    conference_config: ConfigMenu,
    state: ConfigMenuState,
    edit_conference: usize,
}

impl ConferencesTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let items = vec![];

        Self {
            scroll_state: ScrollbarState::default().content_length(icy_board.lock().unwrap().conferences.len()),
            table_state: TableState::default().with_selected(0),
            icy_board: icy_board.clone(),
            in_edit_mode: false,
            conference_config: ConfigMenu { entry: items },
            state: ConfigMenuState::default(),
            edit_conference: 0,
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, mut area: Rect) {
        area.y += 1;
        area.height -= 1;
        frame.render_stateful_widget(
            Scrollbar::default()
                .style(THEME.content_box)
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
        let header = ["", "Keyword", "Display"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(THEME.table_header)
            .height(1);

        let l = self.icy_board.lock().unwrap();
        let rows = l
            .conferences
            .iter()
            .enumerate()
            .map(|(i, cmd)| Row::new(vec![Cell::from(format!("{:-3})", i + 1)), Cell::from(cmd.name.clone())]));
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
            ],
        )
        .header(header)
        .highlight_style(THEME.selected_item)
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
                    self.icy_board.lock().unwrap().conferences.len() - 1
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
                if i + 1 >= self.icy_board.lock().unwrap().conferences.len() {
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
        conf.name = format!("New Conference #{}", self.icy_board.lock().unwrap().conferences.len() + 1);
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
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.conference_config.render(area, frame, &mut self.state);
    }

    fn open_editor(&mut self, conference_number: usize) {
        self.state = ConfigMenuState::default();
        self.edit_conference = conference_number;
        let ib = self.icy_board.lock().unwrap();
        let conf = ib.conferences.get(conference_number).unwrap();
        let items = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "name",
                    format!("Name (#{})", self.table_state.selected().unwrap() + 1),
                    ListValue::Text(25, conf.name.clone()),
                )
                .with_label_width(14),
            ),
            ConfigEntry::Table(
                2,
                vec![
                    ConfigEntry::Item(ListItem::new("is_public", "Public Conference".to_string(), ListValue::Bool(conf.is_public))),
                    ConfigEntry::Item(
                        ListItem::new(
                            "req_sec",
                            "Req. Security if Public".to_string(),
                            ListValue::Text(50, conf.required_security.to_string()),
                        )
                        .with_label_width(28),
                    ),
                ],
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "password",
                    "Password to Join if Private".to_string(),
                    ListValue::Text(25, conf.password.to_string()),
                )
                .with_label_width(28),
            ),
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new("users_menu", "Name/Loc of User's Menu".to_string(), ListValue::Path(conf.users_menu.clone())).with_label_width(28),
            ),
            ConfigEntry::Item(
                ListItem::new("sysop_menu", "Name/Loc of Sysop's Menu".to_string(), ListValue::Path(conf.sysop_menu.clone())).with_label_width(28),
            ),
            ConfigEntry::Item(ListItem::new("news_file", "Name/Loc of NEWS File".to_string(), ListValue::Path(conf.news_file.clone())).with_label_width(28)),
            ConfigEntry::Item(
                ListItem::new(
                    "intro_file",
                    "Name/Loc of Conf INTRO File".to_string(),
                    ListValue::Path(conf.intro_file.clone()),
                )
                .with_label_width(28),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "attachment_location",
                    "Location for Attachments".to_string(),
                    ListValue::Path(conf.attachment_location.clone()),
                )
                .with_label_width(28),
            ),
            ConfigEntry::Item(ListItem::new("command_file", "Conf. CMD.LST File".to_string(), ListValue::Path(conf.command_file.clone())).with_label_width(28)),
            ConfigEntry::Separator,
            ConfigEntry::Table(
                2,
                vec![
                    ConfigEntry::Item(
                        ListItem::new(
                            "pub_upload_dir_file",
                            "Public Upld".to_string(),
                            ListValue::Path(conf.pub_upload_dir_file.clone()),
                        )
                        .with_label_width(12),
                    ),
                    ConfigEntry::Item(
                        ListItem::new("pub_upload_location", ":".to_string(), ListValue::Path(conf.pub_upload_location.clone())).with_label_width(1),
                    ),
                ],
            ),
            ConfigEntry::Separator,
            ConfigEntry::Table(
                2,
                vec![
                    ConfigEntry::Item(ListItem::new("doors_menu", "Doors".to_string(), ListValue::Path(conf.doors_menu.clone())).with_label_width(12)),
                    ConfigEntry::Item(ListItem::new("doors_file", "".to_string(), ListValue::Path(conf.doors_file.clone()))),
                    ConfigEntry::Item(ListItem::new("blt_menu", "Bulletins".to_string(), ListValue::Path(conf.blt_menu.clone())).with_label_width(12)),
                    ConfigEntry::Item(ListItem::new("blt_file", "".to_string(), ListValue::Path(conf.blt_file.clone()))),
                    ConfigEntry::Item(ListItem::new("survey_menu", "Surveys".to_string(), ListValue::Path(conf.survey_menu.clone())).with_label_width(12)),
                    ConfigEntry::Item(ListItem::new("survey_file", "".to_string(), ListValue::Path(conf.survey_file.clone()))),
                    ConfigEntry::Item(ListItem::new("dir_menu", "Directories".to_string(), ListValue::Path(conf.dir_menu.clone())).with_label_width(12)),
                    ConfigEntry::Item(ListItem::new("dir_file", "".to_string(), ListValue::Path(conf.dir_file.clone()))),
                    ConfigEntry::Item(ListItem::new("area_menu", "Areas".to_string(), ListValue::Path(conf.area_menu.clone())).with_label_width(12)),
                    ConfigEntry::Item(ListItem::new("area_file", "".to_string(), ListValue::Path(conf.area_file.clone()))),
                ],
            ),
        ];
        self.conference_config.entry = items;
    }

    fn write_config(&self) {
        let mut ib = self.icy_board.lock().unwrap();
        let conf = &mut ib.conferences[self.edit_conference];

        for item in self.conference_config.iter() {
            self.write_item(item, conf);
        }
    }

    fn write_item(&self, item: &ListItem, conf: &mut icy_board_engine::icy_board::conferences::Conference) {
        match &item.value {
            ListValue::Text(_, text) => match item.id.as_str() {
                "name" => conf.name = text.clone(),
                "password" => conf.password = Password::PlainText(text.clone()),
                "req_sec" => conf.required_security = SecurityExpression::from_str(text).unwrap_or_default(),
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Path(path) => match item.id.as_str() {
                "users_menu" => conf.users_menu = path.clone(),
                "sysop_menu" => conf.sysop_menu = path.clone(),
                "news_file" => conf.news_file = path.clone(),
                "intro_file" => conf.intro_file = path.clone(),
                "attachment_location" => conf.attachment_location = path.clone(),
                "command_file" => conf.command_file = path.clone(),
                "pub_upload_dir_file" => conf.pub_upload_dir_file = path.clone(),
                "pub_upload_location" => conf.pub_upload_location = path.clone(),
                "doors_menu" => conf.doors_menu = path.clone(),
                "doors_file" => conf.doors_file = path.clone(),
                "blt_menu" => conf.blt_menu = path.clone(),
                "blt_file" => conf.blt_file = path.clone(),
                "survey_menu" => conf.survey_menu = path.clone(),
                "survey_file" => conf.survey_file = path.clone(),
                "dir_menu" => conf.dir_menu = path.clone(),
                "dir_file" => conf.dir_file = path.clone(),
                "area_menu" => conf.area_menu = path.clone(),
                "area_file" => conf.area_file = path.clone(),
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Bool(b) => match item.id.as_str() {
                "is_public" => conf.is_public = *b,
                _ => panic!("Unknown id: {}", item.id),
            },
            _ => todo!(),
        }
    }
}

impl TabPage for ConferencesTab {
    fn title(&self) -> String {
        "Conferences".to_string()
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(THEME.content_box)
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
                    self.write_config();
                    return ResultState::default();
                }
                KeyCode::F(2) => {
                    if let Some(item) = self.conference_config.get_item(self.state.selected) {
                        if item.id == "doors_file" || item.id == "blt_file" || item.id == "survey_file" || item.id == "dir_file" || item.id == "area_file" {
                            if let ListValue::Path(path) = &item.value {
                                let path = self.icy_board.lock().unwrap().resolve_file(path);
                                return ResultState {
                                    edit_mode: EditMode::Open(item.id.to_string(), path.clone()),
                                    status_line: String::new(),
                                };
                            }
                        }
                    }
                }
                _ => {
                    self.conference_config.handle_key_press(key, &mut self.state);
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
