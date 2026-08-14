use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::user_base::UserBase;
use icy_board_tui::save_changes_dialog::SaveChangesDialog;
use icy_board_tui::save_changes_dialog::SaveChangesMessage;
use icy_board_tui::tab_page::Page;
use icy_board_tui::tab_page::PageMessage;
use icy_board_tui::theme::get_tui_theme;
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

use super::UserEditor;

/// What the list is ordered by. The file itself keeps its order, this is the view only.
#[derive(Clone, Copy, PartialEq)]
enum SortOrder {
    Record,
    Name,
    Security,
    LastOn,
}

impl SortOrder {
    fn next(self) -> Self {
        match self {
            SortOrder::Record => SortOrder::Name,
            SortOrder::Name => SortOrder::Security,
            SortOrder::Security => SortOrder::LastOn,
            SortOrder::LastOn => SortOrder::Record,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortOrder::Record => "icbsm_sort_record",
            SortOrder::Name => "icbsm_sort_name",
            SortOrder::Security => "icbsm_sort_security",
            SortOrder::LastOn => "icbsm_sort_last_on",
        }
    }
}

pub struct UserList {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,
    save_dialog: Option<SaveChangesDialog>,
    backup: UserBase,
    has_changes: bool,
    in_edit_mode: bool,
    /// Positions in the user base, in the order they are shown.
    view: Vec<usize>,
    search: String,
    searching: bool,
    sort: SortOrder,
}

impl UserList {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let user_len = icy_board.lock().unwrap().users.len();
        let backup = icy_board.lock().unwrap().users.clone();
        let mut list = Self {
            scroll_state: ScrollbarState::default().content_length(user_len),
            table_state: TableState::default().with_selected(if user_len > 0 { 0 } else { usize::MAX }),
            icy_board,
            backup,
            save_dialog: None,
            has_changes: false,
            in_edit_mode: false,
            view: Vec::new(),
            search: String::new(),
            searching: false,
            sort: SortOrder::Record,
        };
        list.rebuild_view();
        list
    }

    fn rebuild_view(&mut self) {
        let board = self.icy_board.lock().unwrap();
        let needle = self.search.trim().to_lowercase();
        let mut view: Vec<usize> = board
            .users
            .iter()
            .enumerate()
            .filter(|(_, user)| needle.is_empty() || user.get_name().to_lowercase().contains(&needle) || user.alias.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect();

        match self.sort {
            SortOrder::Record => {}
            SortOrder::Name => view.sort_by_key(|i| board.users[*i].get_name().to_lowercase()),
            SortOrder::Security => view.sort_by(|a, b| board.users[*b].security_level.cmp(&board.users[*a].security_level)),
            SortOrder::LastOn => view.sort_by(|a, b| board.users[*b].stats.last_on.cmp(&board.users[*a].stats.last_on)),
        }
        drop(board);

        let len = view.len();
        self.view = view;
        self.scroll_state = self.scroll_state.content_length(len);
        if len == 0 {
            self.table_state.select(None);
        } else {
            let selected = self.table_state.selected().unwrap_or(0).min(len - 1);
            self.table_state.select(Some(selected));
            self.scroll_state = self.scroll_state.position(selected);
        }
    }

    /// The record the cursor points at, as a position in the user base.
    fn selected_user(&self) -> Option<usize> {
        self.table_state.selected().and_then(|row| self.view.get(row).copied())
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
        let header = ["", "Name", "Alias", "Sec", "Last On", ""]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(get_tui_theme().table_header)
            .height(1);

        let l = self.icy_board.lock().unwrap();
        let rows = self.view.iter().map(|i| {
            let user = &l.users[*i];
            let last_on = if user.stats.num_times_on == 0 {
                String::new()
            } else {
                user.stats.last_on.format("%Y-%m-%d").to_string()
            };
            let mut marker = String::new();
            if user.flags.delete_flag {
                marker.push('D');
            }
            if user.flags.disabled_flag {
                marker.push('X');
            }
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)).style(get_tui_theme().item),
                Cell::from(user.name.clone()).style(get_tui_theme().item),
                Cell::from(user.alias.clone()).style(get_tui_theme().item),
                Cell::from(user.security_level.to_string()).style(get_tui_theme().item),
                Cell::from(last_on).style(get_tui_theme().item),
                Cell::from(marker).style(get_tui_theme().item),
            ])
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(15 + 1),
                Constraint::Length(3 + 1),
                Constraint::Length(10 + 1),
                Constraint::Length(2),
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
        if self.view.is_empty() {
            return;
        }
        let max = self.view.len();
        let i = match self.table_state.selected() {
            Some(0) | None => max - 1,
            Some(i) => i - 1,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn next(&mut self) {
        if self.view.is_empty() {
            return;
        }
        let max = self.view.len();
        let i = match self.table_state.selected() {
            Some(i) if i + 1 < max => i + 1,
            _ => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn insert(&mut self) {
        use icy_board_engine::icy_board::user_base::{ChatStatus, Password, PasswordInfo, User, UserFlags, UserStats};

        let mut board = self.icy_board.lock().unwrap();
        let new_idx = board.users.len() + 1;
        let new_user = User {
            name: format!("NewUser{new_idx}"),
            password: PasswordInfo {
                password: Password::PlainText("password".into()),
                ..Default::default()
            },
            security_level: 10,
            exp_security_level: 10,
            flags: UserFlags::default(),
            stats: UserStats::default(),
            chat_status: ChatStatus::Available,
            protocol: "Z".into(),
            page_len: 24,
            ..Default::default()
        };
        board.users.new_user(new_user);
        let len = board.users.len();
        drop(board);

        self.search.clear();
        self.sort = SortOrder::Record;
        self.rebuild_view();
        self.scroll_state = self.scroll_state.content_length(len);
        self.table_state.select(Some(self.view.len().saturating_sub(1)));
        self.has_changes = true;
    }

    fn remove(&mut self) {
        if let Some(index) = self.selected_user() {
            let mut board = self.icy_board.lock().unwrap();
            if index < board.users.len() {
                board.users.remove(index);
                drop(board);
                self.rebuild_view();
                self.has_changes = true;
            }
        }
    }

    fn open_save_dialog(&mut self) {
        if self.save_dialog.is_none() {
            self.save_dialog = Some(SaveChangesDialog::new());
        }
    }

    /// Tells the sysop what the list is filtered and sorted by, and which keys do that.
    fn footer(&self) -> String {
        let sort = get_text(self.sort.label());
        if self.searching {
            get_text_args(
                "icbsm_user_list_search",
                std::collections::HashMap::from([("search".to_string(), self.search.clone())]),
            )
        } else if self.search.is_empty() {
            get_text_args("icbsm_user_list_keys", std::collections::HashMap::from([("sort".to_string(), sort)]))
        } else {
            get_text_args(
                "icbsm_user_list_filtered",
                std::collections::HashMap::from([
                    ("search".to_string(), self.search.clone()),
                    ("count".to_string(), self.view.len().to_string()),
                    ("sort".to_string(), sort),
                ]),
            )
        }
    }

    fn handle_search_keys(&mut self, key: KeyEvent) -> bool {
        if !self.searching {
            return false;
        }
        match key.code {
            KeyCode::Esc => {
                self.searching = false;
                self.search.clear();
                self.rebuild_view();
            }
            KeyCode::Enter => self.searching = false,
            KeyCode::Backspace => {
                self.search.pop();
                self.rebuild_view();
            }
            KeyCode::Char(c) => {
                self.search.push(c);
                self.table_state.select(Some(0));
                self.rebuild_view();
            }
            _ => {}
        }
        true
    }

    fn try_save(&mut self) -> PageMessage {
        match self.icy_board.lock().unwrap().save_userbase() {
            Ok(_) => {
                self.has_changes = false;
            }
            Err(e) => {
                log::error!("Failed to save user database: {e}");
            }
        }

        PageMessage::Close
    }

    fn handle_close_request(&mut self) -> PageMessage {
        if self.has_changes {
            self.open_save_dialog();
            PageMessage::None
        } else {
            PageMessage::Close
        }
    }

    fn handle_save_dialog_keys(&mut self, key: KeyEvent) -> Option<PageMessage> {
        if self.save_dialog.is_none() {
            return None;
        }
        let dlg = self.save_dialog.as_mut().unwrap();
        match dlg.handle_key_press(key) {
            SaveChangesMessage::Save => {
                self.save_dialog = None;
                Some(self.try_save())
            }
            SaveChangesMessage::Close => {
                self.save_dialog = None;
                self.has_changes = false;
                // Restore from backup
                self.icy_board.lock().unwrap().users = self.backup.clone();
                Some(PageMessage::Close)
            }
            SaveChangesMessage::Cancel => {
                self.save_dialog = None;
                Some(PageMessage::None)
            }
            SaveChangesMessage::None => Some(PageMessage::None),
        }
    }
}

impl Page for UserList {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title_bottom(self.footer());
        block.render(area, frame.buffer_mut());

        let inner = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.render_table(frame, inner);
        self.render_scrollbar(frame, inner);

        if let Some(dlg) = &mut self.save_dialog {
            dlg.render(frame, area);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        // If dialog is open, it owns the key events.
        if let Some(result) = self.handle_save_dialog_keys(key) {
            // Stop further processing while dialog visible
            if self.save_dialog.is_some() {
                return PageMessage::None;
            }
            if matches!(result, PageMessage::Close) {
                return result;
            }
        }

        if self.handle_search_keys(key) {
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => self.handle_close_request(),
            KeyCode::Up => {
                self.prev();
                PageMessage::None
            }
            KeyCode::Down => {
                self.next();
                PageMessage::None
            }
            KeyCode::Home => {
                if !self.view.is_empty() {
                    self.table_state.select(Some(0));
                    self.scroll_state = self.scroll_state.position(0);
                }
                PageMessage::None
            }
            KeyCode::End => {
                if !self.view.is_empty() {
                    let last = self.view.len() - 1;
                    self.table_state.select(Some(last));
                    self.scroll_state = self.scroll_state.position(last);
                }
                PageMessage::None
            }
            KeyCode::F(3) => {
                self.searching = true;
                self.search.clear();
                self.rebuild_view();
                PageMessage::None
            }
            KeyCode::F(4) => {
                self.sort = self.sort.next();
                self.rebuild_view();
                PageMessage::None
            }
            KeyCode::Insert => {
                self.insert();
                PageMessage::None
            }
            KeyCode::Delete => {
                self.remove();
                PageMessage::None
            }
            KeyCode::F(2) if self.has_changes => self.try_save(),
            KeyCode::Enter => {
                if let Some(index) = self.selected_user() {
                    self.in_edit_mode = true;
                    PageMessage::OpenSubPage(Box::new(UserEditor::new(self.icy_board.clone(), index)))
                } else {
                    PageMessage::None
                }
            }
            _ => PageMessage::None,
        }
    }

    // (Optional) If your Page trait supports this:
    // fn has_control(&self) -> bool {
    //     self.save_dialog.is_some() || self.in_edit_mode
    // }
}
