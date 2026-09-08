use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use icy_board_engine::icy_board::{IcyBoard, commands::Command, menu::Menu};
use icy_board_tui::{
    config_menu::ResultState,
    get_text,
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
    widgets::{Block, BorderType, Borders, Clear, ScrollbarState, TableState, Widget},
};

use crate::edit_command_dialog::{DialogResult, EditCommandDialog, auto_run_value, normalize_table};

pub struct CommandsTab<'a> {
    menu: Arc<Mutex<Menu>>,
    original_commands: Vec<Command>,
    insert_table: InsertTable<'a>,
    edit_cmd_dialog: Option<EditCommandDialog<'a>>,
    visible: Arc<Mutex<Vec<usize>>>,
    filter: Option<String>,
    edit_target: Option<usize>,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl<'a> CommandsTab<'a> {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>) -> Self {
        let original_commands = menu.lock().unwrap().commands.clone();
        let visible = Arc::new(Mutex::new(Vec::<usize>::new()));
        let rows = visible.clone();
        let model = menu.clone();
        let insert_table = InsertTable {
            scroll_state: ScrollbarState::default(),
            table_state: TableState::default(),
            columns: vec![
                Column::new(get_text("command_editor_keyword")).with_width(14),
                Column::new(get_text("mnu_editor_display")).with_width(16),
                Column::new(get_text("mnu_work_actions")).with_width(17),
                Column::new(get_text("command_editor_security")).with_width(15),
                Column::new(get_text("mnu_editor_autorun")),
            ],
            numbered: false,
            get_content: Box::new(move |_, row, column| {
                let index = rows.lock().unwrap().get(*row).copied();
                let menu = model.lock().unwrap();
                let Some(cmd) = index.and_then(|i| menu.commands.get(i)) else {
                    return Line::default();
                };
                match column {
                    0 => Line::from(cmd.keyword.clone()),
                    1 => get_styled_pcb_line(&cmd.display),
                    2 => Line::from(action_summary(cmd)),
                    3 => Line::from(cmd.security.to_string()),
                    4 => Line::from(auto_run_value(&cmd.auto_run).display),
                    _ => Line::default(),
                }
            }),
            content_length: 0,
        };
        let mut tab = Self {
            menu,
            original_commands,
            insert_table,
            visible,
            filter: None,
            edit_cmd_dialog: None,
            edit_target: None,
            icy_board,
        };
        tab.refresh(None);
        tab
    }

    fn selected_index(&self) -> Option<usize> {
        self.insert_table
            .table_state
            .selected()
            .and_then(|row| self.visible.lock().unwrap().get(row).copied())
    }

    fn refresh(&mut self, preferred: Option<usize>) {
        let needle = self.filter.as_deref().unwrap_or_default().to_lowercase();
        let menu = self.menu.lock().unwrap();
        let mut visible = self.visible.lock().unwrap();
        *visible = menu
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                let text = format!(
                    "{} {} {} {} {} {:?}",
                    cmd.keyword,
                    cmd.display,
                    cmd.lighbar_display,
                    action_summary(cmd),
                    cmd.security,
                    cmd.auto_run
                )
                .to_lowercase();
                (needle.is_empty() || text.contains(&needle)).then_some(i)
            })
            .collect();
        if let Some(row) = preferred.and_then(|i| visible.iter().position(|&v| v == i)) {
            self.insert_table.table_state.select(Some(row));
        }
        normalize_table(&mut self.insert_table, visible.len());
    }

    fn open_draft(&mut self, target: Option<usize>, command: Command) {
        self.edit_target = target;
        let index = target.unwrap_or_else(|| self.menu.lock().unwrap().commands.len());
        self.edit_cmd_dialog = Some(EditCommandDialog::new(self.icy_board.clone(), self.menu.clone(), command, index + 1));
    }

    fn remove(&mut self) {
        if let Some(index) = self.selected_index() {
            let mut menu = self.menu.lock().unwrap();
            if index < menu.commands.len() {
                menu.commands.remove(index);
            }
        }
        self.refresh(None);
    }

    fn move_selected(&mut self, down: bool) {
        // Ordering in a filtered view is ambiguous; require the complete list.
        if self.filter.is_some() {
            return;
        }
        let Some(index) = self.selected_index() else {
            return;
        };
        let next = if down { index.checked_add(1) } else { index.checked_sub(1) };
        let len = self.menu.lock().unwrap().commands.len();
        if let Some(next) = next.filter(|&i| i < len && index < len) {
            self.menu.lock().unwrap().commands.swap(index, next);
            self.refresh(Some(next));
        }
    }

    fn hints(&self) -> HotkeyBar {
        if self.filter.is_some() {
            HotkeyBar::new([
                Hotkey::new(KeyCode::Esc, get_text("mnu_work_clear_filter")),
                Hotkey::new(KeyCode::Enter, get_text("mnu_work_edit")),
                Hotkey::new(KeyCode::Insert, get_text("mnu_work_new")),
            ])
        } else {
            HotkeyBar::new([
                Hotkey::new(KeyCode::Insert, get_text("mnu_work_new")),
                Hotkey::new(KeyCode::Enter, get_text("mnu_work_edit")),
                Hotkey::new(KeyCode::Delete, get_text("mnu_work_delete")),
                Hotkey::modified(KeyModifiers::CONTROL, KeyCode::Char('d'), get_text("mnu_work_duplicate")),
                Hotkey::new(KeyCode::Char('/'), get_text("mnu_work_search")),
                Hotkey::alternatives([KeyCode::PageUp, KeyCode::PageDown], get_text("mnu_work_move")),
            ])
        }
    }
}

fn action_summary(command: &Command) -> String {
    command
        .actions
        .iter()
        .map(|a| format!("{} {}", a.command_type, a.parameter))
        .collect::<Vec<_>>()
        .join("; ")
}

impl TabPage for CommandsTab<'_> {
    fn title(&self) -> String {
        get_text("tui_tab_commands")
    }

    fn has_control(&self) -> bool {
        self.edit_cmd_dialog.is_some() || self.filter.is_some()
    }

    fn is_dirty(&self) -> bool {
        self.menu.lock().unwrap().commands != self.original_commands
    }

    fn request_status(&self) -> ResultState {
        if let Some(dialog) = &self.edit_cmd_dialog {
            return ResultState::status_line(dialog.status());
        }
        ResultState::status_line(if self.filter.is_some() {
            get_text("mnu_work_filter_help")
        } else {
            get_text("mnu_work_list_help")
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(dialog) = &mut self.edit_cmd_dialog {
            dialog.ui(frame, area);
            return;
        }
        let selected = self.selected_index();
        self.refresh(selected);
        let area = area.inner(Margin::new(1, 1));
        if area.width < 3 || area.height < 4 {
            return;
        }
        Clear.render(area, frame.buffer_mut());
        Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(get_tui_theme().dialog_box)
            .title(Line::raw(self.title()).style(get_tui_theme().dialog_box_title))
            .render(area, frame.buffer_mut());
        let inner = area.inner(Margin::new(1, 1));
        let hints = self.hints();
        let hint_height = hints.rows(inner.width).len().min(2) as u16;
        let [search, table, footer] = Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(hint_height)]).areas(inner);
        hints.render(footer, frame.buffer_mut());
        if let Some(filter) = &self.filter {
            Line::styled(format!("{}: /{}", get_text("mnu_work_search"), filter), get_tui_theme().filter_text).render(search, frame.buffer_mut());
        } else if let Some(index) = self.selected_index() {
            let menu = self.menu.lock().unwrap();
            if let Some(cmd) = menu.commands.get(index) {
                Line::styled(format!("#{}  {}", index + 1, action_summary(cmd)), get_tui_theme().menu_label).render(search, frame.buffer_mut());
            }
        }
        if table.width > 0 && table.height > 0 {
            self.insert_table.render_table(frame, table);
            if self.insert_table.content_length == 0 && table.height > 2 {
                Line::styled(get_text("mnu_work_empty"), get_tui_theme().table_inactive)
                    .render(Rect::new(table.x, table.y + 2, table.width, 1), frame.buffer_mut());
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if key.kind == KeyEventKind::Release {
            return self.request_status();
        }
        if let Some(dialog) = &mut self.edit_cmd_dialog {
            match dialog.handle_key_press(key) {
                DialogResult::Pending => return self.request_status(),
                DialogResult::Cancelled => {
                    self.edit_cmd_dialog = None;
                }
                DialogResult::Accepted => {
                    let command = dialog.command.lock().unwrap().clone();
                    let mut menu = self.menu.lock().unwrap();
                    let target = if let Some(index) = self.edit_target {
                        if let Some(old) = menu.commands.get_mut(index) {
                            *old = command;
                        }
                        index
                    } else {
                        let index = menu.commands.len();
                        menu.commands.push(command);
                        index
                    };
                    drop(menu);
                    self.edit_cmd_dialog = None;
                    self.refresh(Some(target));
                }
            }
            return self.request_status();
        }
        self.refresh(self.selected_index());
        if key.code == KeyCode::Esc && self.filter.is_some() {
            let selected = self.selected_index();
            self.filter = None;
            self.refresh(selected);
            return self.request_status();
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
            let command = self.selected_index().and_then(|i| self.menu.lock().unwrap().commands.get(i).cloned());
            if let Some(command) = command {
                self.open_draft(None, command);
            }
            return self.request_status();
        }
        if let Some(filter) = &mut self.filter {
            match key.code {
                KeyCode::Char(c) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                    filter.push(c);
                    self.refresh(None);
                    return self.request_status();
                }
                KeyCode::Backspace => {
                    filter.pop();
                    self.refresh(None);
                    return self.request_status();
                }
                KeyCode::Tab | KeyCode::BackTab => return self.request_status(),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Char('/') => {
                self.filter = Some(String::new());
            }
            KeyCode::PageUp => self.move_selected(false),
            KeyCode::PageDown => self.move_selected(true),
            KeyCode::Insert => self.open_draft(None, Command::default()),
            KeyCode::Delete => self.remove(),
            KeyCode::Enter => {
                if let Some(index) = self.selected_index() {
                    let cmd = self.menu.lock().unwrap().commands.get(index).cloned();
                    if let Some(cmd) = cmd {
                        self.open_draft(Some(index), cmd);
                    }
                }
            }
            _ => {
                let _ = self.insert_table.handle_key_press(key);
            }
        }
        self.refresh(self.selected_index());
        self.request_status()
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
