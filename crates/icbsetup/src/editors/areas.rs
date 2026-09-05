use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        message_area::{AreaList, MessageArea},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, ScrollbarState, TableState, Widget},
};

/// One line of a `.NA` area list, and whether the sysop wants it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct NaArea {
    tag: String,
    name: String,
    selected: bool,
}

/// A list written before UTF-8 was common is in the DOS code page.
fn decode_na(data: &[u8]) -> String {
    std::str::from_utf8(data).map_or_else(
        |_| data.iter().map(|byte| codepages::tables::CP437_TO_UNICODE[*byte as usize]).collect(),
        str::to_string,
    )
}

fn parse_na(text: &str) -> Vec<NaArea> {
    let mut seen = HashSet::new();
    text.lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches('\u{1a}').trim();
            if line.is_empty() || line.starts_with([';', '#']) {
                return None;
            }
            let split = line.find(|ch: char| ch.is_whitespace() || ch == ',').unwrap_or(line.len());
            let tag = line[..split].trim().to_ascii_uppercase();
            if tag.is_empty()
                || tag.len() > 32
                || !tag.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
                || !seen.insert(tag.clone())
            {
                return None;
            }
            let name = line[split..].trim().trim_start_matches(',').trim();
            Some(NaArea {
                name: if name.is_empty() { tag.clone() } else { name.to_string() },
                tag,
                selected: true,
            })
        })
        .collect()
}

/// A tag becomes a directory name, so nothing but letters and digits survives.
fn safe_base_name(tag: &str) -> String {
    let mut name = String::with_capacity(tag.len());
    for ch in tag.chars() {
        name.push(if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' });
    }
    name.trim_matches('_').to_string()
}

fn import_na(existing: &mut AreaList, areas: &[NaArea], base_directory: &Path) -> usize {
    let mut tags: HashSet<String> = existing.iter().map(|area| area.ftn_area_tag.to_ascii_uppercase()).collect();
    let mut paths: HashSet<PathBuf> = existing.iter().map(|area| area.path.clone()).collect();
    let mut added = 0;
    for area in areas.iter().filter(|area| area.selected) {
        if tags.contains(&area.tag) {
            continue;
        }
        let stem = safe_base_name(&area.tag);
        if stem.is_empty() {
            continue;
        }
        let mut path = base_directory.join(&stem);
        let mut suffix = 2;
        while paths.contains(&path) {
            path = base_directory.join(format!("{stem}_{suffix}"));
            suffix += 1;
        }
        existing.push(MessageArea {
            name: area.name.clone(),
            ftn_area_tag: area.tag.clone(),
            path: path.clone(),
            ..Default::default()
        });
        tags.insert(area.tag.clone());
        paths.insert(path);
        added += 1;
    }
    added
}

pub struct MessageAreasEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    area_list_orig: AreaList,
    area_list: Arc<Mutex<AreaList>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<AreaList>>)>>,
    import_config_state: ConfigMenuState,
    import_config: Option<ConfigMenu<Arc<Mutex<NaImportSettings>>>>,
    import_settings: Arc<Mutex<NaImportSettings>>,
    import_areas: Option<Vec<NaArea>>,
    import_selected: usize,
    save_dialog: Option<SaveChangesDialog>,
}

#[derive(Clone, Debug, Default)]
struct NaImportSettings {
    source: PathBuf,
    base_directory: PathBuf,
}

impl<'a> MessageAreasEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let area_list_orig = if path.exists() { AreaList::load(&path)? } else { AreaList::default() };
        let area_list = Arc::new(Mutex::new(area_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(area_list_orig.len());
        let content_length = area_list_orig.len();
        let dl2 = area_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("dirs_table_name_header")).with_width(20),
                Column::new(get_text("dirs_table_path_header")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(dl2.lock().unwrap()[*i].name.to_string()),
                    1 => Line::from(format!("{}", dl2.lock().unwrap()[*i].path.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };
        let import_settings = Arc::new(Mutex::new(NaImportSettings {
            source: PathBuf::new(),
            base_directory: path.parent().unwrap_or_else(|| Path::new("")).join("messages"),
        }));

        Ok(Self {
            path: path.clone(),
            insert_table,
            area_list,
            area_list_orig,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            import_config_state: ConfigMenuState::default(),
            import_config: None,
            import_settings,
            import_areas: None,
            import_selected: 0,
            save_dialog: None,
        })
    }

    fn open_import(&mut self) {
        self.import_config_state = ConfigMenuState::default();
        let settings = self.import_settings.lock().unwrap().clone();
        self.import_config = Some(ConfigMenu {
            obj: self.import_settings.clone(),
            entry: vec![
                ConfigEntry::Item(
                    ListItem::new(get_text("area_import_file"), ListValue::Path(settings.source))
                        .with_status(get_text("area_import_file-status"))
                        .with_help(get_text("area_import_file-help"))
                        .with_label_width(18)
                        .with_update_path_value(&|settings: &Arc<Mutex<NaImportSettings>>, value: PathBuf| {
                            settings.lock().unwrap().source = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("area_import_directory"), ListValue::Path(settings.base_directory))
                        .with_status(get_text("area_import_directory-status"))
                        .with_help(get_text("area_import_directory-help"))
                        .with_label_width(18)
                        .with_update_path_value(&|settings: &Arc<Mutex<NaImportSettings>>, value: PathBuf| {
                            settings.lock().unwrap().base_directory = value;
                        }),
                ),
            ],
        });
    }

    fn load_import(&mut self) -> PageMessage {
        let source = self.import_settings.lock().unwrap().source.clone();
        match std::fs::read(&source) {
            Ok(data) => {
                let areas = parse_na(&decode_na(&data));
                if areas.is_empty() {
                    return PageMessage::InfoBox(icy_board_tui::tab_page::InfoState::Warning, get_text("area_import_empty"));
                }
                self.import_areas = Some(areas);
                self.import_selected = 0;
                self.import_config = None;
                PageMessage::None
            }
            Err(err) => PageMessage::InfoBox(
                icy_board_tui::tab_page::InfoState::Error,
                get_text_args("area_import_failed", HashMap::from([("error".to_string(), err.to_string())])),
            ),
        }
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect) {
        let sel = self.insert_table.table_state.selected();
        self.insert_table.render_table(frame, *area);
        self.insert_table.table_state.select(sel);
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected()
            && selected > 0
        {
            let mut levels = self.area_list.lock().unwrap();
            levels.swap(selected, selected - 1);
            self.insert_table.table_state.select(Some(selected - 1));
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected()
            && selected + 1 < self.area_list.lock().unwrap().len()
        {
            let mut levels = self.area_list.lock().unwrap();
            levels.swap(selected, selected + 1);
            self.insert_table.table_state.select(Some(selected + 1));
        }
    }
}

impl<'a> Page for MessageAreasEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("area_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Line::from(Span::from(title).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .title_bottom(Span::styled(get_text("area_editor_key_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 3, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Line::from(
                    Span::from(get_text("area_editor_edit_title")).style(get_tui_theme().dialog_box_title),
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
        if let Some(import_config) = &mut self.import_config {
            let margin = Margin {
                vertical: if area.height >= 18 { 5 } else { 1 },
                horizontal: if area.width >= 70 { 5 } else { 1 },
            };
            let area = area.inner(margin);
            Clear.render(area, frame.buffer_mut());
            Block::new()
                .title_alignment(Alignment::Center)
                .title(Line::from(Span::from(get_text("area_import_title")).style(get_tui_theme().dialog_box_title)))
                .title_bottom(Span::styled(get_text("area_import_load_help"), get_tui_theme().key_binding))
                .style(get_tui_theme().dialog_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .render(area, frame.buffer_mut());
            import_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.import_config_state);
            if let Some(item) = import_config.get_item(self.import_config_state.selected) {
                item.text_field_state.set_cursor_position(frame);
            }
        }
        if let Some(import_areas) = &self.import_areas {
            let margin = Margin {
                vertical: if area.height >= 12 { 2 } else { 0 },
                horizontal: if area.width >= 50 { 3 } else { 0 },
            };
            let area = area.inner(margin);
            Clear.render(area, frame.buffer_mut());
            Block::new()
                .title_alignment(Alignment::Center)
                .title(Line::from(
                    Span::from(get_text("area_import_preview_title")).style(get_tui_theme().dialog_box_title),
                ))
                .title_bottom(Span::styled(get_text("area_import_preview_help"), get_tui_theme().key_binding))
                .style(get_tui_theme().dialog_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .render(area, frame.buffer_mut());
            let inner = area.inner(Margin { vertical: 2, horizontal: 2 });
            let height = inner.height as usize;
            let first = self.import_selected.saturating_sub(height.saturating_sub(1));
            let lines = import_areas
                .iter()
                .enumerate()
                .skip(first)
                .take(height)
                .map(|(index, entry)| {
                    let mark = if entry.selected { "[x]" } else { "[ ]" };
                    let line = Line::from(format!("{mark} {:<32} {}", entry.tag, entry.name));
                    if index == self.import_selected {
                        line.style(get_tui_theme().selected_item)
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>();
            Paragraph::new(Text::from(lines)).render(inner, frame.buffer_mut());
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
                    crate::editors::save_file(&self.path, || self.area_list.lock().unwrap().save(&self.path))
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        if self.import_areas.is_some() {
            let len = self.import_areas.as_ref().map_or(0, Vec::len);
            match key.code {
                KeyCode::Esc => {
                    self.import_areas = None;
                    self.open_import();
                }
                KeyCode::Up => self.import_selected = self.import_selected.saturating_sub(1),
                KeyCode::Down => {
                    if self.import_selected + 1 < len {
                        self.import_selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(entry) = self.import_areas.as_mut().and_then(|areas| areas.get_mut(self.import_selected)) {
                        entry.selected = !entry.selected;
                    }
                }
                KeyCode::Enter => {
                    let areas = self.import_areas.take().unwrap_or_default();
                    let directory = self.import_settings.lock().unwrap().base_directory.clone();
                    let added = import_na(&mut self.area_list.lock().unwrap(), &areas, &directory);
                    self.insert_table.content_length = self.area_list.lock().unwrap().len();
                    return PageMessage::InfoBox(
                        icy_board_tui::tab_page::InfoState::Info,
                        get_text_args("area_import_done", HashMap::from([("count".to_string(), added.to_string())])),
                    );
                }
                _ => {}
            }
            return PageMessage::None;
        }

        if let Some(import_config) = &mut self.import_config {
            if key.code == KeyCode::F(2) {
                return self.load_import();
            }
            let res = import_config.handle_key_press(key, &mut self.import_config_state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.import_config = None;
            }
            return PageMessage::None;
        }

        if let Some(edit_config) = &mut self.edit_config {
            let res = edit_config.handle_key_press(key, &mut self.edit_config_state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.edit_config = None;
                return PageMessage::None;
            }
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => {
                if self.area_list_orig == self.area_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

                KeyCode::Insert => {
                    self.area_list.lock().unwrap().push(MessageArea::default());
                    self.insert_table.content_length += 1;
                }
                KeyCode::F(2) => self.open_import(),
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected()
                        && selected_item < self.area_list.lock().unwrap().len()
                    {
                        self.area_list.lock().unwrap().remove(selected_item);
                        self.insert_table.content_length -= 1;
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.area_list.lock().unwrap();
                        let Some(item) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.area_list.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_name"), ListValue::Text(25, TextFlags::None, item.name.to_string()))
                                        .with_status(get_text("area_editor_name-status"))
                                        .with_help(get_text("area_editor_name-help"))
                                        .with_label_width(16)
                                        .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                            list.lock().unwrap()[*i].name = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_qwk_name"),
                                        ListValue::Text(25, TextFlags::None, item.qwk_name.to_string()),
                                    )
                                    .with_status(get_text("area_editor_qwk_name-status"))
                                    .with_help(get_text("area_editor_qwk_name-help"))
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                        list.lock().unwrap()[*i].qwk_name = value;
                                    }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_fido_tag"),
                                        ListValue::Text(32, TextFlags::None, item.ftn_area_tag.to_string()),
                                    )
                                    .with_status(get_text("area_editor_fido_tag-status"))
                                    .with_help(get_text("area_editor_fido_tag-help"))
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                        list.lock().unwrap()[*i].ftn_area_tag = value.to_ascii_uppercase();
                                    }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_fido_origin"),
                                        ListValue::Text(70, TextFlags::None, item.ftn_origin.to_string()),
                                    )
                                    .with_status(get_text("area_editor_fido_origin-status"))
                                    .with_help(get_text("area_editor_fido_origin-help"))
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: String| {
                                        list.lock().unwrap()[*i].ftn_origin = value;
                                    }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_file"), ListValue::Path(item.path.clone()))
                                        .with_status(get_text("area_editor_file-status"))
                                        .with_help(get_text("area_editor_file-help"))
                                        .with_label_width(16)
                                        .with_update_path_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: PathBuf| {
                                            list.lock().unwrap()[*i].path = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_is_readonly"), ListValue::Bool(item.is_read_only))
                                        .with_status(get_text("area_editor_is_readonly-status"))
                                        .with_help(get_text("area_editor_is_readonly-help"))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: bool| {
                                            list.lock().unwrap()[*i].is_read_only = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("area_editor_allow_aliases"), ListValue::Bool(item.allow_aliases))
                                        .with_status(get_text("area_editor_allow_aliases-status"))
                                        .with_help(get_text("area_editor_allow_aliases-help"))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: bool| {
                                            list.lock().unwrap()[*i].allow_aliases = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_list_sec"),
                                        ListValue::Security(item.req_level_to_list.clone(), item.req_level_to_list.to_string()),
                                    )
                                    .with_status(get_text("area_editor_list_sec-status"))
                                    .with_help(get_text("area_editor_list_sec-help"))
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<AreaList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].req_level_to_list = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_enter_sec"),
                                        ListValue::Security(item.req_level_to_enter.clone(), item.req_level_to_enter.to_string()),
                                    )
                                    .with_status(get_text("area_editor_enter_sec-status"))
                                    .with_help(get_text("area_editor_enter_sec-help"))
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<AreaList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].req_level_to_enter = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_attach_sec"),
                                        ListValue::Security(item.req_level_to_save_attach.clone(), item.req_level_to_save_attach.to_string()),
                                    )
                                    .with_status(get_text("area_editor_attach_sec-status"))
                                    .with_help(get_text("area_editor_attach_sec-help"))
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<AreaList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].req_level_to_save_attach = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("area_editor_qwk_number"),
                                        ListValue::U32(item.qwk_conference_number as u32, 0, u16::MAX as u32),
                                    )
                                    .with_status(get_text("area_editor_qwk_number-status"))
                                    .with_help(get_text("area_editor_qwk_number-help"))
                                    .with_label_width(16)
                                    .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<AreaList>>), value: u32| {
                                        list.lock().unwrap()[*i].qwk_conference_number = value as u16;
                                    }),
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

pub fn edit_areas(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(MessageAreasEditor::new(&path).unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_list_reads_tags_names_comments_and_missing_names() {
        let areas = parse_na(concat!(
            "; comment\r\n",
            "# another comment\r\n",
            "FSX_GEN General discussion\r\n",
            "FSX_BBS\r\n",
            "FSX_TECH,Technical discussion\r\n",
            "fsx_gen duplicate whatever the case\r\n",
            "../SECRET no paths here\r\n",
        ));

        assert_eq!(areas.len(), 3);
        assert_eq!(areas[0].tag, "FSX_GEN");
        assert_eq!(areas[0].name, "General discussion");
        assert_eq!(areas[1].name, "FSX_BBS");
        assert_eq!(areas[2].name, "Technical discussion");
        assert!(areas.iter().all(|area| area.selected));
    }

    #[test]
    fn importing_never_overwrites_an_existing_tag_or_path() {
        let mut existing = AreaList::new(vec![MessageArea {
            name: "Existing".to_string(),
            ftn_area_tag: "FSX_GEN".to_string(),
            path: PathBuf::from("messages/fsx_bbs"),
            ..Default::default()
        }]);
        let areas = parse_na("FSX_GEN Duplicate\r\nFSX_BBS BBS discussion\r\n");

        let added = import_na(&mut existing, &areas, Path::new("messages"));

        assert_eq!(added, 1);
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[1].ftn_area_tag, "FSX_BBS");
        assert_eq!(existing[1].path, PathBuf::from("messages/fsx_bbs_2"));
        assert_eq!(existing[0].name, "Existing");
    }

    #[test]
    fn an_unselected_area_is_not_imported() {
        let mut areas = parse_na("FSX_GEN General\r\nFSX_BBS BBS\r\n");
        areas[0].selected = false;
        let mut existing = AreaList::default();

        let added = import_na(&mut existing, &areas, Path::new("messages"));

        assert_eq!(added, 1);
        assert_eq!(existing[0].ftn_area_tag, "FSX_BBS");
    }

    #[test]
    fn a_tag_becomes_a_portable_base_name() {
        assert_eq!(safe_base_name("FSX.GEN-CHAT"), "fsx_gen_chat");
    }

    #[test]
    fn a_long_area_name_is_not_silently_cut_during_import() {
        let name = "Discussion of bulletin board software and message networks";
        let areas = parse_na(&format!("BBS_SOFT {name}\r\n"));

        assert_eq!(areas[0].name, name);
    }

    #[test]
    fn a_classic_dos_area_list_keeps_its_cp437_names() {
        let text = decode_na(b"FIDO_DE Gr\x81\xe1e aus Deutschland\r\n");
        let areas = parse_na(&text);

        assert_eq!(areas[0].name, "Grüße aus Deutschland");
    }
}
