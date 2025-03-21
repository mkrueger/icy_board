use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::group_list::Group;
use icy_board_engine::icy_board::group_list::GroupList;
use icy_board_tui::config_menu::ConfigEntry;
use icy_board_tui::config_menu::ConfigMenu;
use icy_board_tui::config_menu::ConfigMenuState;
use icy_board_tui::config_menu::ListItem;
use icy_board_tui::config_menu::ListValue;
use icy_board_tui::tab_page::Page;
use icy_board_tui::tab_page::PageMessage;
use icy_board_tui::theme::get_tui_theme;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    text::Text,
    widgets::{Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
};

pub struct GroupEditor {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,
    _old_groups: GroupList,
    in_edit_mode: bool,

    conference_config: ConfigMenu<u32>,
    state: ConfigMenuState,
    edit_conference: usize,
}

impl GroupEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let old_groups = icy_board.lock().unwrap().groups.clone();
        Self {
            scroll_state: ScrollbarState::default().content_length(icy_board.lock().unwrap().conferences.len()),
            table_state: TableState::default().with_selected(0),
            icy_board: icy_board.clone(),
            _old_groups: old_groups,
            in_edit_mode: false,
            conference_config: ConfigMenu { obj: 0, entry: vec![] },
            state: ConfigMenuState::default(),
            edit_conference: 0,
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, mut area: Rect) {
        area.x += 1;
        area.y += 1;
        area.height -= 1;
        frame.render_stateful_widget(
            Scrollbar::default()
                .style(get_tui_theme().dialog_box_scrollbar)
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
        let header = ["", "Name", "#Users"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(get_tui_theme().table_header)
            .height(1);

        let l = self.icy_board.lock().unwrap();
        let rows = l.groups.iter().enumerate().map(|(i, group)| {
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)).style(get_tui_theme().item),
                Cell::from(group.name.clone()).style(get_tui_theme().item),
                Cell::from(group.members.len().to_string()).style(get_tui_theme().item),
            ])
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(4 + 1),
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
                    self.icy_board.lock().unwrap().groups.len() - 1
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
                if i + 1 >= self.icy_board.lock().unwrap().groups.len() {
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
        let mut group = Group::default();
        group.name = format!("new_group{}", self.icy_board.lock().unwrap().groups.len() + 1);
        self.icy_board.lock().unwrap().groups.push(group);
        self.scroll_state = self.scroll_state.content_length(self.icy_board.lock().unwrap().groups.len());
        self.save_groups();
    }

    fn remove(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if i > 0 {
                self.icy_board.lock().unwrap().groups.remove(i);
                let len = self.icy_board.lock().unwrap().groups.len();
                self.scroll_state = self.scroll_state.content_length(len);

                if len >= i - 1 {
                    self.table_state.select(Some(i - 1));
                } else {
                    self.table_state.select(Some(0));
                }
            }
        }
        self.save_groups();
    }

    fn render_editor(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.conference_config.render(area, frame, &mut self.state);
    }

    fn open_editor(&mut self, index: usize) {
        self.state = ConfigMenuState::default();
        self.edit_conference = index;
        let ib = self.icy_board.lock().unwrap();
        let group = ib.groups.get(index).unwrap();
        let items = vec![
            ConfigEntry::Item(ListItem::new("Name".to_string(), ListValue::Text(60, group.name.to_string())).with_label_width(14)),
            ConfigEntry::Item(ListItem::new("Members".to_string(), ListValue::Text(60, group.members.join(",").to_string())).with_label_width(14)),
        ];
        self.conference_config.entry = items;
    }

    fn save_groups(&self) {
        let path = self.icy_board.lock().unwrap().config.paths.group_file.clone();
        let path = self.icy_board.lock().unwrap().resolve_file(&path);
        let _ = self.icy_board.lock().unwrap().groups.save(&path);
    }
}

impl Page for GroupEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });

        if self.in_edit_mode {
            self.render_editor(frame, area);
            //self.set_cursor_position(frame);
            return;
        }

        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    } /*
    fn set_cursor_position(&self, frame: &mut Frame) {
    self.conference_config
    .get_item(self.state.selected)
    .unwrap()
    .text_field_state
    .set_cursor_position(frame);
    }*/

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.in_edit_mode {
            let res = self.conference_config.handle_key_press(key, &mut self.state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.in_edit_mode = false;
                return PageMessage::None;
            }
            self.save_groups();
            return PageMessage::None;
        }
        match key.code {
            KeyCode::Esc => {
                return PageMessage::Close;
            }
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Insert => self.insert(),
            KeyCode::Delete => self.remove(),
            KeyCode::Enter => {
                if let Some(state) = self.table_state.selected() {
                    self.in_edit_mode = true;
                    self.open_editor(state);
                    return PageMessage::None;
                    //return ResultState::status_line(String::new());
                } else {
                    self.in_edit_mode = false;
                }
            }
            _ => {}
        }
        return PageMessage::None;
    }
}
