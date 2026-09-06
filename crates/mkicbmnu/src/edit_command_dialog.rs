use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard,
        commands::{ActionTrigger, AutoRun, Command, CommandAction, CommandType},
        menu::Menu,
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    pcb_line::{get_styled_pcb_line, get_styled_pcb_line_with_highlight},
    position_editor::PositionEditor,
    theme::get_tui_theme,
};
use icy_engine::TextPane;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditCommandMode {
    Config,
    Table,
}

fn auto_run_value(auto_run: &AutoRun) -> ComboBoxValue {
    let key = match auto_run {
        AutoRun::Disabled => "mnu_editor_autorun_disabled",
        AutoRun::FirstCmd => "mnu_editor_autorun_first",
        AutoRun::Every => "mnu_editor_autorun_every",
        AutoRun::After => "mnu_editor_autorun_after",
        AutoRun::Loop => "mnu_editor_autorun_loop",
    };
    ComboBoxValue::new(get_text(key), format!("{auto_run:?}"))
}

pub struct EditCommandDialog<'a> {
    pub command: Arc<Mutex<Command>>,
    id: usize,
    mode: EditCommandMode,

    state: ConfigMenuState,
    config: ConfigMenu<Arc<Mutex<Command>>>,

    insert_table: InsertTable<'a>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<Command>>)>>,
}

impl<'a> EditCommandDialog<'a> {
    pub(crate) fn new(icy_board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>, command: Command, id: usize) -> Self {
        let info_width = 16;

        let command_arc = Arc::new(Mutex::new(command.clone()));
        let cmd3 = command_arc.clone();
        let disp_file = menu.lock().unwrap().display_file.clone();
        let file = icy_board.lock().unwrap().resolve_file(&disp_file);

        let buffer = if file.exists() {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("ans");
            icy_engine::FileFormat::from_extension(ext)
                .unwrap_or(icy_engine::FileFormat::Ansi)
                .load(&file, None)
                .unwrap()
                .screen
                .buffer
        } else {
            icy_engine::TextBuffer::new((80, 25))
        };

        let position_editor = Arc::new(Mutex::new(PositionEditor { buffer }));

        let pos_ed = position_editor.clone();
        let items = vec![
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    get_text("mnu_editor_display_text"),
                    ListValue::Text(25, TextFlags::None, command.display.clone()),
                )
                .with_status(get_text("mnu_editor_display_text_status"))
                .with_label_width(info_width)
                .with_update_text_value(&|cmd: &Arc<Mutex<Command>>, value: String| {
                    cmd.lock().unwrap().display = value;
                }),
            ),
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    get_text("mnu_editor_highlighted_text"),
                    ListValue::Text(25, TextFlags::None, command.lighbar_display.clone()),
                )
                .with_status(get_text("mnu_editor_highlighted_text_status"))
                .with_label_width(info_width)
                .with_update_text_value(&|cmd: &Arc<Mutex<Command>>, value: String| {
                    cmd.lock().unwrap().lighbar_display = value;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("mnu_editor_position"),
                    ListValue::Position(
                        Box::new(move |frame, pos| {
                            let size = pos_ed.lock().unwrap().buffer.size();
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
                .with_status(get_text("mnu_editor_position_status"))
                .with_label_width(info_width)
                .with_update_value(Box::new(|cmd: &Arc<Mutex<Command>>, value: &ListValue| {
                    if let ListValue::Position(_, _, pos) = value {
                        cmd.lock().unwrap().position = *pos;
                    }
                })),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("command_editor_keyword"),
                    ListValue::Text(10, TextFlags::None, command.keyword.to_string()),
                )
                .with_status(get_text("mnu_editor_keyword_status"))
                .with_label_width(info_width)
                .with_update_text_value(&|cmd: &Arc<Mutex<Command>>, value: String| {
                    cmd.lock().unwrap().keyword = value;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("mnu_editor_autorun"),
                    ListValue::ComboBox(ComboBox {
                        cur_value: auto_run_value(&command.auto_run),
                        selected_item: 0,
                        first_item: 0,
                        is_edit_open: false,
                        values: AutoRun::iter().map(|x| auto_run_value(&x)).collect::<Vec<ComboBoxValue>>(),
                    }),
                )
                .with_status(get_text("mnu_editor_autorun_status"))
                .with_label_width(info_width)
                .with_update_combobox_value(&|cmd: &Arc<Mutex<Command>>, value: &ComboBox| {
                    if let Ok(auto_run) = AutoRun::from_str(&value.cur_value.value) {
                        cmd.lock().unwrap().auto_run = auto_run;
                    }
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("mnu_editor_time"), ListValue::U32(command.autorun_time as u32, 0, 3600))
                    .with_status(get_text("mnu_editor_time_status"))
                    .with_label_width(info_width)
                    .with_update_u32_value(&|cmd: &Arc<Mutex<Command>>, value: u32| {
                        cmd.lock().unwrap().autorun_time = value as u64;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("mnu_editor_help_file"), ListValue::Text(25, TextFlags::None, command.help.clone()))
                    .with_status(get_text("mnu_editor_help_file_status"))
                    .with_label_width(info_width)
                    .with_update_text_value(&|cmd: &Arc<Mutex<Command>>, value: String| {
                        cmd.lock().unwrap().help = value;
                    }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("command_editor_security"),
                    ListValue::Security(command.security.clone(), command.security.to_string()),
                )
                .with_status(get_text("mnu_editor_security_status"))
                .with_label_width(info_width)
                .with_update_sec_value(&|cmd: &Arc<Mutex<Command>>, value: SecurityExpression| {
                    cmd.lock().unwrap().security = value;
                }),
            ),
        ];

        let cmd2 = command_arc.clone();
        let content_length = cmd2.lock().unwrap().actions.len().max(1);
        let insert_table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(command_arc.lock().unwrap().actions.len()),
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("command_editor_command_type")).with_width(20),
                Column::new(get_text("command_editor_header_parameter")),
            ],
            numbered: false,
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
            command: command_arc.clone(),
            id,
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                obj: command_arc,
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
                }
                EditCommandMode::Table => match key.code {
                    KeyCode::Char('1') => {
                        if let Some(selected) = self.insert_table.table_state.selected()
                            && selected > 0
                        {
                            let mut menu = self.command.lock().unwrap();
                            menu.actions.swap(selected, selected - 1);
                            self.insert_table.table_state.select(Some(selected - 1));
                        }
                    }
                    KeyCode::Char('2') => {
                        if let Some(selected) = self.insert_table.table_state.selected()
                            && selected + 1 < self.command.lock().unwrap().actions.len()
                        {
                            let mut menu = self.command.lock().unwrap();
                            menu.actions.swap(selected, selected + 1);
                            self.insert_table.table_state.select(Some(selected + 1));
                        }
                    }

                    KeyCode::Insert => {
                        self.command.lock().unwrap().actions.push(CommandAction::default());
                        self.insert_table.content_length = self.command.lock().unwrap().actions.len();
                        self.insert_table.scroll_state = self.insert_table.scroll_state.content_length(self.insert_table.content_length);
                    }
                    KeyCode::Delete => {
                        if let Some(selected_item) = self.insert_table.table_state.selected()
                            && selected_item < self.command.lock().unwrap().actions.len()
                        {
                            self.command.lock().unwrap().actions.remove(selected_item);
                            self.insert_table.content_length = self.command.lock().unwrap().actions.len();
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
                            let mut first_item = 0;
                            let values = CommandType::iter()
                                .enumerate()
                                .map(|(i, x)| {
                                    if x == action.command_type {
                                        first_item = i.saturating_sub(4);
                                    }
                                    ComboBoxValue::new(format!("{}", x), format!("{:?}", x))
                                })
                                .collect();

                            self.edit_config = Some(ConfigMenu {
                                obj: (selected_item, self.command.clone()),
                                entry: vec![
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("command_editor_command_type"),
                                            ListValue::ComboBox(ComboBox {
                                                selected_item,
                                                first_item: 0,
                                                cur_value: ComboBoxValue::new(format!("{}", action.command_type), format!("{:?}", action.command_type)),
                                                values,
                                                is_edit_open: false,
                                            }),
                                        )
                                        .with_label_width(16)
                                        .with_update_combobox_value(
                                            &|(i, cmd): &(usize, Arc<Mutex<Command>>), value: &ComboBox| {
                                                if let Ok(command_type) = CommandType::from_str(&value.cur_value.value) {
                                                    cmd.lock().unwrap().actions[*i].command_type = command_type;
                                                }
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("command_editor_parameter"), ListValue::Text(10, TextFlags::None, parameter))
                                            .with_status(get_text("mnu_editor_parameter_status"))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, cmd): &(usize, Arc<Mutex<Command>>), value: String| {
                                                cmd.lock().unwrap().actions[*i].parameter = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("mnu_editor_run_on_selection"),
                                            ListValue::Bool(action.trigger == ActionTrigger::Selection),
                                        )
                                        .with_status(get_text("mnu_editor_run_on_selection_status"))
                                        .with_label_width(16)
                                        .with_update_bool_value(
                                            &|(i, cmd): &(usize, Arc<Mutex<Command>>), value: bool| {
                                                cmd.lock().unwrap().actions[*i].trigger =
                                                    if value { ActionTrigger::Selection } else { ActionTrigger::Activation };
                                            },
                                        ),
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

    pub fn ui(&mut self, frame: &mut Frame, screen: Rect) {
        let area = screen.inner(Margin::new(2, 2));
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Line::from(
                Span::from(format!(
                    " {} ",
                    get_text_args("mnu_editor_command_title", HashMap::from([("id".to_string(), self.id.to_string())]))
                ))
                .style(get_tui_theme().dialog_box_title),
            ))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let vertical = Layout::vertical([Constraint::Length(9), Constraint::Fill(1)]);
        let [header, footer] = vertical.areas(area.inner(Margin { vertical: 1, horizontal: 1 }));
        self.display_insert_table(frame, &area, &footer);

        let sel = self.state.selected;
        if self.mode == EditCommandMode::Table {
            self.state.selected = usize::MAX;
        }
        self.config.render(header, frame, &mut self.state);

        // POSITION EDIT (full screen editor)
        if self.state.selected == 2 {
            return;
        }
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
        /*
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
        }*/
        self.state.selected = sel;
        self.display_insert_table(frame, &area, &footer);
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
                .title(Line::from(
                    Span::from(format!(" {} ", get_text("mnu_editor_edit_action"))).style(get_tui_theme().dialog_box_title),
                ))
                .style(get_tui_theme().dialog_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
            block.render(area, frame.buffer_mut());
            edit_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);
            edit_config
                .get_item(self.edit_config_state.selected)
                .unwrap()
                .text_field_state
                .set_cursor_position(frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autorun_choices_localize_display_but_preserve_parseable_values() {
        for (auto_run, key) in AutoRun::iter().zip([
            "mnu_editor_autorun_disabled",
            "mnu_editor_autorun_first",
            "mnu_editor_autorun_every",
            "mnu_editor_autorun_after",
            "mnu_editor_autorun_loop",
        ]) {
            let value = auto_run_value(&auto_run);
            assert_eq!(value.display, get_text(key));
            assert_eq!(AutoRun::from_str(&value.value).unwrap(), auto_run);
        }
    }
}
