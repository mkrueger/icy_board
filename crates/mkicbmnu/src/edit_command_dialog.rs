use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        commands::{ActionTrigger, AutoRun, Command, CommandAction, CommandType},
        menu::Menu,
        IcyBoard,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    insert_table::InsertTable,
    pcb_line::{get_styled_pcb_line, get_styled_pcb_line_with_highlight},
    position_editor::PositionEditor,
    theme::THEME,
};
use icy_engine::TextPane;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
    Frame,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditCommandMode {
    Config,
    Table,
}

pub struct EditCommandDialog<'a> {
    pub command: Arc<Mutex<Command>>,
    id: usize,
    mode: EditCommandMode,

    state: ConfigMenuState,
    config: ConfigMenu,

    insert_table: InsertTable<'a>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> EditCommandDialog<'a> {
    pub(crate) fn new(icy_board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>, command: Command, id: usize) -> Self {
        let info_width = 16;

        let command_arc = Arc::new(Mutex::new(command.clone()));
        let cmd3 = command_arc.clone();
        let file = icy_board.lock().unwrap().resolve_file(&menu.lock().unwrap().display_file);

        let buffer = if file.exists() {
            icy_engine::Buffer::load_buffer(&file, true).unwrap()
        } else {
            icy_engine::Buffer::new((80, 25))
        };

        let position_editor = Arc::new(Mutex::new(PositionEditor { buffer }));

        let pos_ed = position_editor.clone();
        let items = vec![
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new("text", "Display Text".to_string(), ListValue::Text(25, command.display.clone()))
                    .with_status("Text displayed.")
                    .with_label_width(info_width),
            ),
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    "highlight_text",
                    "Highlighted Text".to_string(),
                    ListValue::Text(25, command.lighbar_display.clone()),
                )
                .with_status("Text displayed, when highlighted.")
                .with_label_width(info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "position",
                    "Position".to_string(),
                    ListValue::Position(
                        Box::new(move |frame, pos| {
                            let size = pos_ed.lock().unwrap().buffer.get_size();
                            let area = Rect::new(
                                (frame.area().width - size.width as u16) / 2,
                                (frame.area().height - size.height as u16) / 2,
                                size.width as u16,
                                size.height as u16,
                            );

                            pos_ed.lock().unwrap().ui(frame, pos, area);

                            for c in menu.lock().unwrap().commands.iter() {
                                if c.display == cmd3.lock().unwrap().display {
                                    continue;
                                };
                                let position_line = get_styled_pcb_line(&c.display);
                                let line_area = Rect::new(area.x + c.position.x, area.y + c.position.y, position_line.width() as u16, 1);
                                position_line.render(line_area, frame.buffer_mut());
                            }
                            for c in menu.lock().unwrap().commands.iter() {
                                if c.display != cmd3.lock().unwrap().display {
                                    continue;
                                }
                                let position_line = get_styled_pcb_line_with_highlight(&c.display, true);
                                let line_area = Rect::new(area.x + pos.x, area.y + pos.y, position_line.width() as u16, 1);
                                position_line.render(line_area, frame.buffer_mut());
                            }
                        }),
                        Box::new(move |evt, pos| position_editor.lock().unwrap().handle_event(evt, pos)),
                        command.position,
                    ),
                )
                .with_status("The help file to display.")
                .with_label_width(info_width),
            ),
            ConfigEntry::Item(
                ListItem::new("keyword", "Keyword".to_string(), ListValue::Text(10, command.keyword.to_string()))
                    .with_status("The help file to display.")
                    .with_label_width(info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "autorun",
                    "Autorun".to_string(),
                    ListValue::ComboBox(ComboBox {
                        cur_value: ComboBoxValue::new(format!("{:?}", command.auto_run), format!("{:?}", command.auto_run)),
                        first: 0,
                        scroll_state: ScrollbarState::default(),
                        values: AutoRun::iter()
                            .map(|x| ComboBoxValue::new(format!("{:?}", x), format!("{:?}", x)))
                            .collect::<Vec<ComboBoxValue>>(),
                    }),
                )
                .with_status("The type of the menu.")
                .with_label_width(info_width),
            ),
            ConfigEntry::Item(
                ListItem::new("autorun_time", "Time".to_string(), ListValue::U32(command.autorun_time as u32, 0, 3600))
                    .with_status("Autorun after a specific amount of time.")
                    .with_label_width(info_width),
            ),
        ];

        let cmd2 = command_arc.clone();
        let content_length = cmd2.lock().unwrap().actions.len().max(1);
        let insert_table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(command_arc.lock().unwrap().actions.len()),
            table_state: TableState::default().with_selected(0),
            headers: vec!["Command Type    ".to_string(), "Parameter".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().actions.len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{:?}", cmd2.lock().unwrap().actions[*i].command_type)),
                    1 => Line::from(cmd2.lock().unwrap().actions[*i].parameter.clone()),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Self {
            command: command_arc,
            id,
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                entry: vec![ConfigEntry::Group(String::new(), items)],
            },
            insert_table,
            mode: EditCommandMode::Config,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
        }
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> Res<bool> {
        if let Some(edit_config) = &mut self.edit_config {
            match key.code {
                KeyCode::Esc => {
                    for item in &edit_config.entry {
                        if let ConfigEntry::Item(item) = item {
                            match item.id.as_str() {
                                "command_type" => {
                                    if let ListValue::ComboBox(ref value) = item.value {
                                        let value = value.cur_value.value.parse::<CommandType>().unwrap();
                                        let selected_item = self.insert_table.table_state.selected().unwrap();
                                        let action = &mut self.command.lock().unwrap().actions[selected_item];
                                        action.command_type = value;
                                    }
                                }
                                "parameter" => {
                                    if let ListValue::Text(_, ref value) = item.value {
                                        let selected_item = self.insert_table.table_state.selected().unwrap();
                                        let action = &mut self.command.lock().unwrap().actions[selected_item];
                                        action.parameter = value.clone();
                                    }
                                }
                                "run_on_selection" => {
                                    if let ListValue::Bool(ref value) = item.value {
                                        let selected_item = self.insert_table.table_state.selected().unwrap();
                                        let action = &mut self.command.lock().unwrap().actions[selected_item];
                                        action.trigger = if *value { ActionTrigger::Selection } else { ActionTrigger::Activation };
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    self.edit_config = None;
                    return Ok(true);
                }
                _ => {
                    edit_config.handle_key_press(key, &mut self.edit_config_state);
                }
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Esc => {
                return Ok(false);
            }

            KeyCode::Tab => {
                if self.mode == EditCommandMode::Config {
                    self.mode = EditCommandMode::Table;
                } else if self.mode == EditCommandMode::Table {
                    self.mode = EditCommandMode::Config;
                }
            }

            _ => match self.mode {
                EditCommandMode::Config => {
                    self.config.handle_key_press(key, &mut self.state);
                    self.write_back_values();
                }
                EditCommandMode::Table => match key.code {
                    KeyCode::Char('1') => {
                        if let Some(selected) = self.insert_table.table_state.selected() {
                            if selected > 0 {
                                let mut menu = self.command.lock().unwrap();
                                menu.actions.swap(selected, selected - 1);
                                self.insert_table.table_state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Char('2') => {
                        if let Some(selected) = self.insert_table.table_state.selected() {
                            if selected + 1 < self.command.lock().unwrap().actions.len() {
                                let mut menu = self.command.lock().unwrap();
                                menu.actions.swap(selected, selected + 1);
                                self.insert_table.table_state.select(Some(selected + 1));
                            }
                        }
                    }

                    KeyCode::Insert => {
                        self.command.lock().unwrap().actions.push(CommandAction::default());
                        self.insert_table.content_length = self.command.lock().unwrap().actions.len();
                        self.insert_table.scroll_state = self.insert_table.scroll_state.content_length(self.insert_table.content_length);
                    }
                    KeyCode::Delete => {
                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            if selected_item < self.command.lock().unwrap().actions.len() {
                                self.command.lock().unwrap().actions.remove(selected_item);
                                self.insert_table.content_length = self.command.lock().unwrap().actions.len();
                            }
                        }
                    }

                    KeyCode::Enter => {
                        self.edit_config_state = ConfigMenuState::default();

                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            let cmd = self.command.lock().unwrap();
                            let Some(action) = cmd.actions.get(selected_item) else {
                                return Ok(true);
                            };
                            let parameter = action.parameter.clone();
                            let mut first = 0;
                            let values = CommandType::iter()
                                .enumerate()
                                .map(|(i, x)| {
                                    if x == action.command_type {
                                        first = i.saturating_sub(4);
                                    }
                                    ComboBoxValue::new(format!("{}", x), format!("{:?}", x))
                                })
                                .collect();

                            self.edit_config = Some(ConfigMenu {
                                entry: vec![
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            "command_type",
                                            "Command Type".to_string(),
                                            ListValue::ComboBox(ComboBox {
                                                first,
                                                scroll_state: ScrollbarState::default().content_length(CommandType::iter().count()),
                                                cur_value: ComboBoxValue::new(format!("{}", action.command_type), format!("{:?}", action.command_type)),
                                                values,
                                            }),
                                        )
                                        .with_label_width(16),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new("parameter", "Parameter".to_string(), ListValue::Text(10, parameter))
                                            .with_status("The help file to display.")
                                            .with_label_width(16),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            "run_on_selection",
                                            "Run on Selection".to_string(),
                                            ListValue::Bool(action.trigger == ActionTrigger::Selection),
                                        )
                                        .with_status("The help file to display.")
                                        .with_label_width(16),
                                    ),
                                ],
                            });
                        } else {
                            self.insert_table.handle_key_press(key)?;
                        }
                    }
                    _ => {
                        self.insert_table.handle_key_press(key)?;
                    }
                },
            },
        }
        Ok(true)
    }

    fn write_back_values(&mut self) {
        let ConfigEntry::Group(_, items) = &self.config.entry[0] else {
            return;
        };
        for entry in items {
            if let ConfigEntry::Item(item) = entry {
                match item.id.as_str() {
                    "text" => {
                        if let ListValue::Text(_, ref value) = item.value {
                            self.command.lock().unwrap().display = value.clone();
                        }
                    }
                    "highlight_text" => {
                        if let ListValue::Text(_, ref value) = item.value {
                            self.command.lock().unwrap().lighbar_display = value.clone();
                        }
                    }
                    "position" => {
                        if let ListValue::Position(_, _, ref value) = item.value {
                            self.command.lock().unwrap().position = value.clone();
                        }
                    }
                    "keyword" => {
                        if let ListValue::Text(_, ref value) = item.value {
                            self.command.lock().unwrap().keyword = value.clone();
                        }
                    }
                    "autorun" => {
                        if let ListValue::ComboBox(ref value) = item.value {
                            let value = value.cur_value.value.parse::<AutoRun>().unwrap();
                            self.command.lock().unwrap().auto_run = value;
                        }
                    }
                    "autorun_time" => {
                        if let ListValue::U32(value, _, _) = item.value {
                            self.command.lock().unwrap().autorun_time = value as u64;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn ui(&mut self, frame: &mut Frame, screen: Rect) {
        let area = screen.inner(Margin::new(2, 2));
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(format!(" Command ID {} ", self.id)).style(THEME.content_box_title)))
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let vertical = Layout::vertical([Constraint::Length(9), Constraint::Fill(1)]);
        let [header, footer] = vertical.areas(area.inner(Margin { vertical: 1, horizontal: 1 }));
        if self.state.in_edit {
            self.display_insert_table(frame, &area, &footer);
        }

        let sel = self.state.selected;
        if self.mode == EditCommandMode::Table {
            self.state.selected = usize::MAX;
        }
        self.config.render(header, frame, &mut self.state);

        if self.state.in_edit {
            // POSITION EDIT (full screen editor)
            if self.state.selected == 2 {
                return;
            }
            self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
        }

        for i in self.config.iter() {
            if i.id == "text" {
                if let ListValue::Text(_, ref value) = &i.value {
                    let mut area = header;
                    area.x += 19;
                    area.width -= 19;
                    area.height = 1;

                    get_styled_pcb_line(value).render(area, frame.buffer_mut());
                }
            }
            if i.id == "highlight_text" {
                if let ListValue::Text(_, ref value) = &i.value {
                    let mut area = header;
                    area.x += 19;
                    area.width -= 19;
                    area.y += 2;
                    area.height = 1;
                    get_styled_pcb_line(value).render(area, frame.buffer_mut());
                }
            }
        }
        self.state.selected = sel;
        if !self.state.in_edit {
            self.display_insert_table(frame, &area, &footer);
        }
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect, footer: &Rect) {
        let sel = self.insert_table.table_state.selected();
        if self.mode == EditCommandMode::Config {
            self.insert_table.table_state.select(None);
        }
        self.insert_table.render_table(frame, *footer);
        self.insert_table.table_state.select(sel);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 6, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" Edit Action ").style(THEME.content_box_title)))
                .style(THEME.content_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
            block.render(area, frame.buffer_mut());
            edit_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);

            if self.edit_config_state.in_edit {
                edit_config
                    .get_item(self.edit_config_state.selected)
                    .unwrap()
                    .text_field_state
                    .set_cursor_position(frame);
            }
        }
    }
}
