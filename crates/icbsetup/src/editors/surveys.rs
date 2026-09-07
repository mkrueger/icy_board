use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        security_expr::SecurityExpression,
        surveys::{Survey, SurveyList},
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
};

pub struct SurveyEditor<'a> {
    path: std::path::PathBuf,
    survey_list_orig: SurveyList,
    insert_table: InsertTable<'a>,
    survey_list: Arc<Mutex<SurveyList>>,

    detail: super::EditorDialog<(usize, Arc<Mutex<SurveyList>>)>,
    save_changes: super::EditorSaveChanges,
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

            columns: vec![
                Column::new(get_text("survey_editor_editor_header_question")).with_width(30),
                Column::new(get_text("survey_editor_editor_header_answer")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].survey_file.display())),
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].answer_file.display())),
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
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }

    fn with_path_base(mut self, path_base: PathBuf) -> Self {
        self.detail.state.path_base = Some(path_base);
        self
    }
}

impl<'a> Page for SurveyEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("surveys_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = super::list_editor_frame(
            title,
            get_text("icb_setup_key_conf_list_help"),
            self.detail.is_open() || self.save_changes.is_open(),
        );
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let area = area.inner(Margin { vertical: 8, horizontal: 3 });
            self.detail.render(frame, area, get_text("survey_editor_editor"), String::new());
        }
        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            crate::editors::save_file(&self.path, || self.survey_list.lock().unwrap().save(&self.path))
        }) {
            return message;
        }

        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.survey_list_orig != *self.survey_list.lock().unwrap());
            }
            _ => match key.code {
                KeyCode::PageUp => self.insert_table.move_row(&mut self.survey_list.lock().unwrap(), -1),
                KeyCode::PageDown => self.insert_table.move_row(&mut self.survey_list.lock().unwrap(), 1),

                KeyCode::Insert => {
                    self.insert_table.push_row(&mut *self.survey_list.lock().unwrap(), Survey::default());
                }
                KeyCode::Delete => {
                    self.insert_table.remove_row(&mut *self.survey_list.lock().unwrap());
                }

                KeyCode::Enter => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.survey_list.lock().unwrap();
                        let Some(action) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.detail.open(ConfigMenu {
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

pub fn edit_surveys(board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    let root = board.1.lock().unwrap().root_path.clone();
    PageMessage::OpenSubPage(Box::new(SurveyEditor::new(&path).unwrap().with_path_base(root)))
}
