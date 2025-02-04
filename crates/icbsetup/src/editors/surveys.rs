use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        security_expr::SecurityExpression,
        surveys::{Survey, SurveyList},
        IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    insert_table::InsertTable,
    tab_page::Editor,
    theme::get_tui_theme,
};
use ratatui::{
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
    Frame,
};

pub struct SurveyEditor<'a> {
    path: std::path::PathBuf,
    survey_list: SurveyList,

    insert_table: InsertTable<'a>,
    surveys: Arc<Mutex<Vec<Survey>>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> SurveyEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let surveys = if path.exists() { SurveyList::load(&path)? } else { SurveyList::default() };
        let command_arc = Arc::new(Mutex::new(surveys.surveys.clone()));
        let scroll_state = ScrollbarState::default().content_length(surveys.surveys.len());
        let content_length = surveys.surveys.len();
        let cmd2 = command_arc.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec!["".to_string(), "Question            ".to_string(), "Answer".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", *i + 1)),
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].survey_file.display())),
                    2 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].answer_file.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            survey_list: surveys,
            insert_table,
            surveys: command_arc,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
        })
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect) {
        let sel = self.insert_table.table_state.selected();
        self.insert_table.render_table(frame, *area);
        self.insert_table.table_state.select(sel);
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected > 0 {
                let mut levels = self.surveys.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.surveys.lock().unwrap().len() {
                let mut levels = self.surveys.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Editor for SurveyEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" Surveys ").style(get_tui_theme().content_box_title)))
            .style(get_tui_theme().content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 9, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(Span::from(" Edit Survey ").style(get_tui_theme().content_box_title)))
                .style(get_tui_theme().content_box)
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

    fn handle_key_press(&mut self, key: KeyEvent) -> bool {
        if let Some(edit_config) = &mut self.edit_config {
            match key.code {
                KeyCode::Esc => {
                    let Some(selected_item) = self.insert_table.table_state.selected() else {
                        return true;
                    };
                    for item in edit_config.iter() {
                        match item.id.as_str() {
                            "survey" => {
                                if let ListValue::Path(path) = &item.value {
                                    self.surveys.lock().unwrap()[selected_item].survey_file = path.clone();
                                }
                            }
                            "answer" => {
                                if let ListValue::Path(path) = &item.value {
                                    self.surveys.lock().unwrap()[selected_item].answer_file = path.clone();
                                }
                            }
                            "security" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    if let Ok(expr) = SecurityExpression::from_str(text) {
                                        self.surveys.lock().unwrap()[selected_item].required_security = expr;
                                    }
                                }
                            }
                            _ => {
                                panic!("Unknown item: {}", item.id);
                            }
                        }
                    }
                    self.edit_config = None;
                    return true;
                }
                _ => {
                    edit_config.handle_key_press(key, &mut self.edit_config_state);
                }
            }
            return true;
        }

        match key.code {
            KeyCode::Esc => {
                self.survey_list.surveys.clear();
                self.survey_list.surveys.append(&mut self.surveys.lock().unwrap());
                self.survey_list.save(&self.path).unwrap();
                return false;
            }
            _ => match key.code {
                KeyCode::Char('1') => self.move_up(),
                KeyCode::Char('2') => self.move_down(),

                KeyCode::Insert => {
                    self.surveys.lock().unwrap().push(Survey::default());
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.surveys.lock().unwrap().len() {
                            self.surveys.lock().unwrap().remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.surveys.lock().unwrap();
                        let Some(action) = cmd.get(selected_item) else {
                            return true;
                        };
                        self.edit_config = Some(ConfigMenu {
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new("survey", "Survey File".to_string(), ListValue::Path(action.survey_file.clone())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("answer", "Answer File".to_string(), ListValue::Path(action.answer_file.clone())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("security", "Security".to_string(), ListValue::Text(25, action.required_security.to_string()))
                                        .with_label_width(16),
                                ),
                            ],
                        });
                    } else {
                        self.insert_table.handle_key_press(key).unwrap();
                    }
                }

                _ => {
                    self.insert_table.handle_key_press(key).unwrap();
                }
            },
        }
        true
    }
}
