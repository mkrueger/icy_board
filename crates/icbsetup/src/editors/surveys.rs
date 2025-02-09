use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        security_expr::SecurityExpression,
        surveys::{Survey, SurveyList},
        IcyBoard, IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    get_text, get_text_args,
    insert_table::InsertTable,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
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
    survey_list_orig: SurveyList,
    insert_table: InsertTable<'a>,
    survey_list: Arc<Mutex<SurveyList>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<SurveyList>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> SurveyEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let survey_list_orig = if path.exists() { SurveyList::load(&path)? } else { SurveyList::default() };
        let survey_list = Arc::new(Mutex::new(survey_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(survey_list_orig.surveys.len());
        let content_length = survey_list_orig.len();
        let cmd2 = survey_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),

            headers: vec![
                "".to_string(),
                format!("{:<30}", get_text("survey_editor_editor_header_question")),
                format!("{:<30}", get_text("survey_editor_editor_header_answer")),
            ],
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
            survey_list_orig,
            insert_table,
            survey_list,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            save_dialog: None,
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
                let mut levels = self.survey_list.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.survey_list.lock().unwrap().len() {
                let mut levels = self.survey_list.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Page for SurveyEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("surveys_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(title).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 8, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("survey_editor_editor")).style(get_tui_theme().dialog_box_title),
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
        if let Some(save_changes) = &self.save_dialog {
            save_changes.render(frame, area);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.save_dialog.is_some() {
            let res = self.save_dialog.as_mut().unwrap().handle_key_press(key);
            return match res {
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Cancel => {
                    self.save_dialog = None;
                    PageMessage::None
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Close => PageMessage::Close,
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Save => {
                    if let Some(parent) = self.path.parent() {
                        if !parent.exists() {
                            std::fs::create_dir_all(parent).unwrap();
                        }
                    }
                    self.survey_list.lock().unwrap().save(&self.path).unwrap();
                    PageMessage::Close
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        if let Some(edit_config) = &mut self.edit_config {
            match key.code {
                KeyCode::Esc => {
                    self.edit_config = None;
                    return PageMessage::None;
                }
                _ => {
                    edit_config.handle_key_press(key, &mut self.edit_config_state);
                }
            }
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => {
                if self.survey_list_orig == self.survey_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

                KeyCode::Insert => {
                    self.survey_list.lock().unwrap().push(Survey::default());
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.survey_list.lock().unwrap().len() {
                            self.survey_list.lock().unwrap().remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.survey_list.lock().unwrap();
                        let Some(action) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.survey_list.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(get_text("survey_editor_editor_file"), ListValue::Path(action.survey_file.clone()))
                                        .with_label_width(16)
                                        .with_update_path_value(&|(i, list): &(usize, Arc<Mutex<SurveyList>>), value: PathBuf| {
                                            list.lock().unwrap()[*i].survey_file = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("survey_editor_editor_answer_file"), ListValue::Path(action.answer_file.clone()))
                                        .with_label_width(16)
                                        .with_update_path_value(&|(i, list): &(usize, Arc<Mutex<SurveyList>>), value: PathBuf| {
                                            list.lock().unwrap()[*i].answer_file = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("survey_editor_editor_security"),
                                        ListValue::Security(action.required_security.clone(), action.required_security.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<SurveyList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].required_security = value;
                                        },
                                    ),
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
        PageMessage::None
    }
}

pub fn edit_surveys(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(SurveyEditor::new(&path).unwrap()))
}
