use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::group_list::Group;
use icy_board_engine::icy_board::group_list::GroupList;
use icy_board_tui::chrome::{dim_background, dirty_title, frame_title};
use icy_board_tui::config_menu::ConfigEntry;
use icy_board_tui::config_menu::ConfigMenu;
use icy_board_tui::config_menu::ConfigMenuState;
use icy_board_tui::config_menu::ListItem;
use icy_board_tui::config_menu::ListValue;
use icy_board_tui::config_menu::TextFlags;
use icy_board_tui::hotkeys::HotkeyBar;
use icy_board_tui::tab_page::{InfoState, Page, PageMessage};
use icy_board_tui::theme::{config_title, get_tui_theme};
use icy_board_tui::{get_text, get_text_args};
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
    edit_backup: Option<GroupList>,
    in_edit_mode: bool,

    conference_config: ConfigMenu<(usize, Arc<Mutex<IcyBoard>>)>,
    state: ConfigMenuState,
    edit_conference: usize,
}

impl GroupEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let group_len = icy_board.lock().unwrap().groups.len();
        Self {
            scroll_state: ScrollbarState::default().content_length(group_len),
            table_state: TableState::default().with_selected(if group_len > 0 { 0 } else { usize::MAX }),
            icy_board: icy_board.clone(),
            edit_backup: None,
            in_edit_mode: false,
            conference_config: ConfigMenu {
                obj: (0, icy_board.clone()),
                entry: vec![],
            },
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
            .map(|title| Cell::from(Text::from(Vec::from(config_title(title)))))
            .collect::<Row>()
            .height(2);

        let l = self.icy_board.lock().unwrap();
        let rows = l.groups.iter().enumerate().map(|(i, group)| {
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)),
                Cell::from(group.name.clone()),
                Cell::from(group.members.len().to_string()),
            ])
            .style(get_tui_theme().table)
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
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn prev(&mut self) {
        if self.icy_board.lock().unwrap().groups.is_empty() {
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
        self.scroll_state = self.scroll_state.position(i);
    }

    fn next(&mut self) {
        if self.icy_board.lock().unwrap().groups.is_empty() {
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
        self.scroll_state = self.scroll_state.position(i);
    }

    fn insert(&mut self) -> PageMessage {
        let original = self.icy_board.lock().unwrap().groups.clone();
        let group = Group {
            name: format!("new_group{}", self.icy_board.lock().unwrap().groups.len() + 1),
            ..Default::default()
        };
        self.icy_board.lock().unwrap().groups.push(group);
        self.scroll_state = self.scroll_state.content_length(self.icy_board.lock().unwrap().groups.len());
        match self.save_groups() {
            Ok(()) => PageMessage::None,
            Err(err) => {
                self.icy_board.lock().unwrap().groups = original;
                PageMessage::InfoBox(InfoState::Error, save_error(err))
            }
        }
    }

    fn remove(&mut self) -> PageMessage {
        let original = self.icy_board.lock().unwrap().groups.clone();
        if let Some(i) = self.table_state.selected()
            && i > 0
        {
            self.icy_board.lock().unwrap().groups.remove(i);
            let len = self.icy_board.lock().unwrap().groups.len();
            self.scroll_state = self.scroll_state.content_length(len);

            if len >= i - 1 {
                self.table_state.select(Some(i - 1));
            } else {
                self.table_state.select(Some(0));
            }
        }
        match self.save_groups() {
            Ok(()) => PageMessage::None,
            Err(err) => {
                self.icy_board.lock().unwrap().groups = original;
                PageMessage::InfoBox(InfoState::Error, save_error(err))
            }
        }
    }

    fn render_editor(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.conference_config.render(area, frame, &mut self.state);
    }

    fn open_editor(&mut self, index: usize) -> bool {
        self.state = ConfigMenuState::default();
        self.edit_conference = index;
        let ib = self.icy_board.lock().unwrap();
        let Some(group) = ib.groups.get(index) else {
            // An empty list has nothing to edit.
            return false;
        };
        self.edit_backup = Some(ib.groups.clone());
        let items = vec![
            ConfigEntry::Item(
                ListItem::new("Name".to_string(), ListValue::Text(60, TextFlags::None, group.name.to_string()))
                    .with_label_width(14)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().groups[*i].name = value;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new("Members".to_string(), ListValue::Text(60, TextFlags::None, group.members.join(",").to_string()))
                    .with_label_width(14)
                    .with_update_text_value(&|(i, board): &(usize, Arc<Mutex<IcyBoard>>), value: String| {
                        board.lock().unwrap().groups[*i].members = value.split(',').map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).collect();
                    }),
            ),
        ];
        drop(ib);
        self.conference_config.obj = (index, self.icy_board.clone());
        self.conference_config.entry = items;
        true
    }

    fn save_groups(&self) -> icy_board_engine::Res<()> {
        let path = self.icy_board.lock().unwrap().config.paths.group_file.clone();
        let path = self.icy_board.lock().unwrap().resolve_file(&path);
        self.icy_board.lock().unwrap().groups.save(&path)
    }
}

fn save_error(err: impl std::fmt::Display) -> String {
    get_text_args("icbsm_save_failed", std::collections::HashMap::from([("error".to_string(), err.to_string())]))
}

impl Page for GroupEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let mut block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(frame_title(get_text("icbsm_menu_groups"), get_tui_theme().dialog_box_title));
        if !self.in_edit_mode {
            block = block.title_bottom(HotkeyBar::for_id("icbsm_menu_keys").line());
        }
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { vertical: 1, horizontal: 1 });

        self.render_table(frame, area);
        self.render_scrollbar(frame, area);

        if self.in_edit_mode {
            // Paint the list first; dim it before clearing and painting the form.
            let backdrop = frame.area();
            dim_background(frame.buffer_mut(), backdrop);
            let popup = super::preferences::centered(area, area.width, 6.min(area.height));
            let dirty = self
                .edit_backup
                .as_ref()
                .and_then(|backup| backup.get(self.edit_conference))
                .is_some_and(|backup| {
                    self.icy_board
                        .lock()
                        .unwrap()
                        .groups
                        .get(self.edit_conference)
                        .is_some_and(|group| group.name != backup.name || group.members != backup.members)
                });
            Clear.render(popup, frame.buffer_mut());
            Block::new()
                .style(get_tui_theme().dialog_box)
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(frame_title(dirty_title(get_text("icbsm_menu_groups"), dirty), get_tui_theme().dialog_box_title))
                .title_bottom(HotkeyBar::for_id("icbsm_group_edit_keys").line())
                .render(popup, frame.buffer_mut());
            self.render_editor(frame, popup);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.in_edit_mode {
            let res = self.conference_config.handle_key_press(key, &mut self.state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.in_edit_mode = false;
                return match self.save_groups() {
                    Ok(()) => {
                        self.edit_backup = None;
                        PageMessage::None
                    }
                    Err(err) => {
                        if let Some(groups) = self.edit_backup.take() {
                            self.icy_board.lock().unwrap().groups = groups;
                        }
                        PageMessage::InfoBox(InfoState::Error, save_error(err))
                    }
                };
            }
            return PageMessage::None;
        }
        match key.code {
            KeyCode::Esc => {
                return PageMessage::Close;
            }
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Insert => return self.insert(),
            KeyCode::Delete => return self.remove(),
            KeyCode::Enter => {
                if let Some(state) = self.table_state.selected() {
                    self.in_edit_mode = self.open_editor(state);
                    return PageMessage::None;
                    //return ResultState::status_line(String::new());
                } else {
                    self.in_edit_mode = false;
                }
            }
            _ => {}
        }
        PageMessage::None
    }
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn group_form_is_painted_above_a_dimmed_list_at_classic_size() {
        let mut board = IcyBoard::default();
        board.groups.push(Group {
            name: "Sysops".into(),
            members: vec!["Alice".into()],
            ..Default::default()
        });
        let mut editor = GroupEditor::new(Arc::new(Mutex::new(board)));
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        let mut expected = terminal.backend().buffer().clone();
        let theme = get_tui_theme();
        assert_eq!(expected[(12, 4)].fg, theme.selected_item.fg.unwrap());
        assert_eq!(expected[(12, 4)].bg, theme.selected_item.bg.unwrap());
        let area = expected.area;
        dim_background(&mut expected, area);

        assert!(editor.open_editor(0));
        editor.in_edit_mode = true;
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        let actual = terminal.backend().buffer();
        for x in 3..77 {
            assert_eq!(actual[(x, 4)], expected[(x, 4)]);
        }
        // The six-row form is centered in the existing 74×21 list interior.
        assert_eq!(actual[(3, 9)].style(), theme.dialog_box.underline_color(ratatui::style::Color::Reset));
        let text: String = (0..80).map(|x| actual[(x, 10)].symbol()).collect();
        assert!(text.contains("Sysops"));
        let footer: String = (0..80).map(|x| actual[(x, 14)].symbol()).collect();
        assert!(footer.contains(&HotkeyBar::for_id("icbsm_group_edit_keys").line().to_string()));
    }
}
