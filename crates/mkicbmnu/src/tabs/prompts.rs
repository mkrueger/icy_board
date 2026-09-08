use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use icy_board_engine::icy_board::{IcyBoard, menu::Menu};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    hotkeys::{Hotkey, HotkeyBar},
    insert_table::{Column, InsertTable},
    pcb_line::get_styled_pcb_line,
    tab_page::TabPage,
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, ScrollbarState, TableState, Widget, Wrap},
};

type Prompts = Vec<(String, String)>;
type Entry = Arc<Mutex<(String, String)>>;

struct Draft {
    index: Option<usize>,
    base: Prompts,
    entry: Entry,
    config: ConfigMenu<Entry>,
    state: ConfigMenuState,
    raw: bool,
}

/// The stored suffix selects a language file set, so compare it the way the
/// board normalizes its language list: lowercase, without a leading dot.
fn language_key(suffix: &str) -> String {
    suffix.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn language_choices(board: &IcyBoard) -> Vec<ComboBoxValue> {
    board
        .languages
        .iter()
        .filter(|language| !language.extension.trim().is_empty())
        .map(|language| ComboBoxValue::new(format!("{} (.{})", language.description, language.extension), language.extension.clone()))
        .collect()
}

fn language_label(board: &IcyBoard, suffix: &str) -> String {
    let key = language_key(suffix);
    board
        .languages
        .iter()
        .find(|language| language_key(&language.extension) == key)
        .map_or_else(|| suffix.to_string(), |language| format!("{} (.{})", language.description, language.extension))
}

fn draft_form(entry: Entry, value: &(String, String), choices: &[ComboBoxValue], raw: bool) -> ConfigMenu<Entry> {
    let width = 18;
    let language = if raw || choices.is_empty() {
        ListValue::Text(20, TextFlags::None, value.0.clone())
    } else {
        let mut values = choices.to_vec();
        // Keep an imported suffix spelled exactly as stored, even when the
        // board writes the same language differently.
        match values.iter_mut().find(|choice| language_key(&choice.value) == language_key(&value.0)) {
            Some(choice) => choice.value = value.0.clone(),
            None => values.insert(0, ComboBoxValue::new(value.0.clone(), value.0.clone())),
        }
        let selected = values.iter().position(|choice| choice.value == value.0).unwrap_or(0);
        ListValue::ComboBox(ComboBox {
            cur_value: values[selected].clone(),
            selected_item: selected,
            first_item: selected.saturating_sub(2),
            is_edit_open: false,
            values,
        })
    };
    ConfigMenu {
        obj: entry,
        entry: vec![
            ConfigEntry::Item(
                ListItem::new(get_text("mnu_prompts_language"), language)
                    .with_label_width(width)
                    .with_status(get_text("mnu_prompts_language_status"))
                    .with_update_value(Box::new(|entry: &Entry, value: &ListValue| match value {
                        ListValue::Text(_, _, text) => entry.lock().unwrap().0 = text.clone(),
                        ListValue::ComboBox(combo) => entry.lock().unwrap().0 = combo.cur_value.value.clone(),
                        _ => {}
                    })),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("mnu_prompts_text"), ListValue::Text(u16::MAX, TextFlags::None, value.1.clone()))
                    .with_label_width(width)
                    .with_status(get_text("mnu_prompts_text_status"))
                    .with_update_text_value(&|entry: &Entry, text: String| entry.lock().unwrap().1 = text),
            ),
        ],
    }
}

/// Language-specific prompt overrides, not command prompts or follow-up
/// questions. `Menu::import_pcboard` keeps the first, suffix-less prompt in
/// `Menu::prompt` and stores the remaining (suffix, prompt) pairs here. The
/// current engine's `input_menu_prompt` only prints `Menu::prompt`.
pub struct PromptsTab<'a> {
    board: Arc<Mutex<IcyBoard>>,
    menu: Arc<Mutex<Menu>>,
    original: Prompts,
    observed: Prompts,
    rows: Arc<Mutex<Prompts>>,
    table: InsertTable<'a>,
    draft: Option<Draft>,
    delete: Option<(usize, Prompts)>,
    message: Option<String>,
}

impl PromptsTab<'_> {
    pub fn new(board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>) -> Self {
        let original = menu.lock().unwrap().prompts.clone();
        let rows = Arc::new(Mutex::new(original.clone()));
        let content = rows.clone();
        let labels = board.clone();
        let table = InsertTable {
            scroll_state: ScrollbarState::default().content_length(original.len()),
            table_state: TableState::default().with_selected((!original.is_empty()).then_some(0)),
            columns: vec![
                Column::new(get_text("mnu_prompts_language")).with_width(30),
                Column::new(get_text("mnu_prompts_text")),
            ],
            numbered: true,
            get_content: Box::new(move |_, row, column| {
                let rows = content.lock().unwrap();
                let Some((language, prompt)) = rows.get(*row) else {
                    return Line::default();
                };
                match column {
                    0 => Line::from(language_label(&labels.lock().unwrap(), language)),
                    1 => get_styled_pcb_line(prompt),
                    _ => Line::default(),
                }
            }),
            content_length: original.len(),
        };
        let mut tab = Self {
            board,
            menu,
            observed: original.clone(),
            original,
            rows,
            table,
            draft: None,
            delete: None,
            message: None,
        };
        tab.refresh_from_menu();
        tab
    }

    /// Also called by render/input. Undo replaces the Menu inside the same
    /// Arc. An open draft is never discarded or silently rebased: applying
    /// against a changed prompts vector reports a conflict instead.
    pub fn refresh_from_menu(&mut self) {
        self.observed = self.menu.lock().unwrap().prompts.clone();
        self.rows.lock().unwrap().clone_from(&self.observed);
        self.table.content_length = self.observed.len();
        self.table.scroll_state = self.table.scroll_state.content_length(self.observed.len());
        let selection = if self.observed.is_empty() {
            None
        } else {
            Some(self.table.table_state.selected().unwrap_or(0).min(self.observed.len() - 1))
        };
        self.table.table_state.select(selection);
    }

    fn begin_edit(&mut self, index: Option<usize>) {
        let value = index.and_then(|index| self.observed.get(index)).cloned().unwrap_or_default();
        let entry = Arc::new(Mutex::new(value.clone()));
        let choices = language_choices(&self.board.lock().unwrap());
        self.draft = Some(Draft {
            index,
            base: self.observed.clone(),
            config: draft_form(entry.clone(), &value, &choices, false),
            entry,
            state: ConfigMenuState::default(),
            raw: false,
        });
        self.message = None;
    }

    fn toggle_raw(&mut self) {
        let choices = language_choices(&self.board.lock().unwrap());
        if let Some(draft) = &mut self.draft {
            flush(&draft.config);
            draft.raw = !draft.raw;
            let value = draft.entry.lock().unwrap().clone();
            draft.config = draft_form(draft.entry.clone(), &value, &choices, draft.raw);
        }
    }

    fn apply_draft(&mut self) {
        let Some(draft) = &self.draft else {
            return;
        };
        flush(&draft.config);
        let value = draft.entry.lock().unwrap().clone();
        let mut menu = self.menu.lock().unwrap();
        if menu.prompts != draft.base {
            self.message = Some(get_text("mnu_prompts_conflict"));
            return;
        }
        // Preserve imported legacy keys byte-for-byte if only the text is
        // edited. Validate new/renamed keys without normalizing existing data.
        let unchanged_key = draft.index.is_some_and(|index| draft.base[index].0 == value.0);
        if !unchanged_key {
            if language_key(&value.0).is_empty()
                || value
                    .0
                    .chars()
                    .any(|ch| ch.is_whitespace() || ch.is_control() || matches!(ch, ',' | '/' | '\\'))
            {
                self.message = Some(get_text("mnu_prompts_invalid_extension"));
                return;
            }
            if menu
                .prompts
                .iter()
                .enumerate()
                .any(|(index, (language, _))| Some(index) != draft.index && language_key(language) == language_key(&value.0))
            {
                self.message = Some(get_text("mnu_prompts_duplicate"));
                return;
            }
        }
        let index = if let Some(index) = draft.index {
            menu.prompts[index] = value;
            index
        } else {
            menu.prompts.push(value);
            menu.prompts.len() - 1
        };
        drop(menu);
        self.draft = None;
        self.message = None;
        self.refresh_from_menu();
        self.table.table_state.select(Some(index));
    }

    fn confirm_delete(&mut self) {
        if let Some((index, base)) = self.delete.take() {
            let mut menu = self.menu.lock().unwrap();
            if menu.prompts != base {
                self.message = Some(get_text("mnu_prompts_conflict"));
                return;
            }
            menu.prompts.remove(index);
        }
        self.refresh_from_menu();
    }

    fn combo_open(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| {
            draft
                .config
                .get_item(draft.state.selected)
                .is_some_and(|item| matches!(&item.value, ListValue::ComboBox(combo) if combo.is_edit_open))
        })
    }

    fn render_draft(&mut self, frame: &mut Frame, area: Rect) {
        let status = self.message.clone().unwrap_or_else(|| {
            let changed = self.draft.as_ref().is_some_and(|draft| draft.base != self.observed);
            get_text(if changed { "mnu_prompts_conflict" } else { "mnu_prompts_edit_status" })
        });
        let Some(draft) = &mut self.draft else {
            return;
        };
        let hints = HotkeyBar::new([
            Hotkey::alternatives([KeyCode::F(2), KeyCode::F(10)], get_text("mnu_prompts_apply")),
            Hotkey::new(KeyCode::Esc, get_text("mnu_prompts_discard")),
            Hotkey::alternatives([KeyCode::Up, KeyCode::Down], get_text("mnu_prompts_field")),
            Hotkey::new(KeyCode::F(3), get_text("mnu_prompts_raw")),
        ]);
        let inner = dialog(frame, area, 9, get_text("mnu_prompts_edit_title"), hints);
        if inner.width < 8 || inner.height < 3 {
            return;
        }
        let [fields, note] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(inner);
        draft.config.render(fields, frame, &mut draft.state);
        if let Some(item) = draft.config.get_item(draft.state.selected) {
            item.text_field_state.set_cursor_position(frame);
        }
        if note.height > 0 {
            let text = Paragraph::new(status).style(get_tui_theme().menu_label).wrap(Wrap { trim: true });
            let height = text.line_count(note.width).min(usize::from(note.height)) as u16;
            text.render(Rect::new(note.x, note.y, note.width, height), frame.buffer_mut());
        }
    }
}

/// ConfigMenu publishes text edits while rendering; flush explicitly so a key
/// sequence without an intervening frame stores the same values.
fn flush(config: &ConfigMenu<Entry>) {
    for item in config.iter() {
        if let Some(update) = &item.update_value {
            update(&config.obj, &item.value);
        }
    }
}

fn dialog(frame: &mut Frame, area: Rect, height: u16, title: String, hints: HotkeyBar) -> Rect {
    let width = area.width.min(76);
    let height = area.height.min(height);
    let area = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);
    Clear.render(area, frame.buffer_mut());
    Block::new()
        .style(get_tui_theme().background)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(get_tui_theme().dialog_box)
        .title(Line::raw(title).style(get_tui_theme().dialog_box_title))
        .title_bottom(hints.line())
        .render(area, frame.buffer_mut());
    area.inner(Margin { horizontal: 2, vertical: 1 })
}

impl TabPage for PromptsTab<'_> {
    fn title(&self) -> String {
        get_text("mnu_prompts_tab")
    }

    fn has_control(&self) -> bool {
        self.draft.is_some() || self.delete.is_some()
    }

    fn is_dirty(&self) -> bool {
        self.menu.lock().unwrap().prompts != self.original
    }

    fn request_status(&self) -> ResultState {
        let key = if self.delete.is_some() {
            "mnu_prompts_delete_question"
        } else if let Some(draft) = &self.draft {
            if draft.base != self.observed {
                "mnu_prompts_conflict"
            } else {
                "mnu_prompts_edit_status"
            }
        } else {
            "mnu_prompts_status"
        };
        ResultState::status_line(self.message.clone().unwrap_or_else(|| get_text(key)))
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.refresh_from_menu();
        let mut block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(get_tui_theme().dialog_box)
            .title(Line::raw(self.title()).style(get_tui_theme().dialog_box_title));
        if !self.has_control() {
            block = block.title_bottom(
                HotkeyBar::new([
                    Hotkey::alternatives([KeyCode::Up, KeyCode::Down], get_text("mnu_prompts_move")),
                    Hotkey::new(KeyCode::Enter, get_text("mnu_prompts_edit")),
                    Hotkey::new(KeyCode::Insert, get_text("mnu_prompts_new")),
                    Hotkey::new(KeyCode::Delete, get_text("mnu_prompts_delete")),
                    Hotkey::new(KeyCode::F(1), get_text("mnu_prompts_help")),
                ])
                .line(),
            );
        }
        block.render(area, frame.buffer_mut());
        let inner = area.inner(Margin { horizontal: 2, vertical: 1 });
        if inner.height < 3 || inner.width < 12 {
            return;
        }
        // The suffix-less main prompt lives on the General page; showing it
        // here explains what these language entries actually override.
        let [default, table] = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(inner);
        Line::styled(get_text("mnu_prompts_default"), get_tui_theme().config_title).render(default, frame.buffer_mut());
        let value = Rect::new(default.x, default.y + 1, default.width, 1);
        let prompt = self.menu.lock().unwrap().prompt.clone();
        if prompt.trim().is_empty() {
            Line::styled(get_text("mnu_prompts_default_empty"), get_tui_theme().table_inactive).render(value, frame.buffer_mut());
        } else {
            get_styled_pcb_line(&prompt).render(value, frame.buffer_mut());
        }
        if self.observed.is_empty() {
            Line::styled(get_text("mnu_prompts_empty"), get_tui_theme().table_inactive).render(table, frame.buffer_mut());
        } else if table.width > 2 && table.height > 2 {
            self.table.render_table(frame, table);
        }
        self.render_draft(frame, area);
        if let Some((index, base)) = &self.delete {
            let inner = dialog(
                frame,
                area,
                8,
                get_text("mnu_prompts_delete_title"),
                HotkeyBar::new([
                    Hotkey::new(KeyCode::Enter, get_text("mnu_prompts_delete")),
                    Hotkey::new(KeyCode::Esc, get_text("mnu_prompts_cancel")),
                ]),
            );
            let entry = get_text_args(
                "mnu_prompts_delete_entry",
                std::collections::HashMap::from([
                    ("language".to_string(), language_label(&self.board.lock().unwrap(), &base[*index].0)),
                    ("prompt".to_string(), base[*index].1.clone()),
                ]),
            );
            Paragraph::new(format!("{}\n\n{entry}", get_text("mnu_prompts_delete_question")))
                .style(get_tui_theme().item)
                .wrap(Wrap { trim: false })
                .render(Rect::new(inner.x, inner.y, inner.width, inner.height.min(4)), frame.buffer_mut());
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        self.refresh_from_menu();
        if key.kind == KeyEventKind::Release {
            return self.request_status();
        }
        if self.delete.is_some() {
            if key.modifiers.is_empty() {
                match key.code {
                    KeyCode::Enter => self.confirm_delete(),
                    KeyCode::Esc => self.delete = None,
                    _ => {}
                }
            }
            return self.request_status();
        }
        if self.draft.is_some() {
            // Terminal chords such as Ctrl+Enter must not reach the form and
            // open a choice list that then swallows the apply key.
            if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                return self.request_status();
            }
            // The open choice list owns Escape and Enter first.
            let open = self.combo_open();
            if !open {
                match key.code {
                    KeyCode::Esc => {
                        self.draft = None;
                        self.message = None;
                        return self.request_status();
                    }
                    // Ctrl+Enter is intercepted by many terminals, so apply with a function key.
                    KeyCode::F(2) | KeyCode::F(10) => {
                        self.apply_draft();
                        return self.request_status();
                    }
                    KeyCode::F(3) => {
                        self.toggle_raw();
                        return self.request_status();
                    }
                    _ => {}
                }
            }
            if let Some(draft) = &mut self.draft {
                draft.config.handle_key_press(key, &mut draft.state);
                flush(&draft.config);
                self.message = None;
            }
            return self.request_status();
        }
        if !key.modifiers.is_empty() {
            return self.request_status();
        }
        self.message = None;
        match key.code {
            KeyCode::Insert => self.begin_edit(None),
            KeyCode::Enter => {
                if let Some(index) = self.table.table_state.selected() {
                    self.begin_edit(Some(index));
                }
            }
            KeyCode::Delete => {
                if let Some(index) = self.table.table_state.selected().filter(|index| *index < self.observed.len()) {
                    self.delete = Some((index, self.observed.clone()));
                }
            }
            _ => {
                let _ = self.table.handle_key_press(key);
            }
        }
        self.request_status()
    }
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
