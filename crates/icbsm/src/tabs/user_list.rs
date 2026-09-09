use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::user_base::{User, UserBase};
use icy_board_engine::icy_board::user_store::UserUpdateError;
use icy_board_tui::chrome::{dim_background, dirty_title, frame_title};
use icy_board_tui::hotkeys::{Hotkey, HotkeyBar};
use icy_board_tui::save_changes_dialog::SaveChangesDialog;
use icy_board_tui::save_changes_dialog::SaveChangesMessage;
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
    text::{Line, Span, Text},
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
            SortOrder::Record => "icbsm_list_sort_record",
            SortOrder::Name => "icbsm_list_sort_name",
            SortOrder::Security => "icbsm_list_sort_security",
            SortOrder::LastOn => "icbsm_list_sort_last_on",
        }
    }
}

pub struct UserList {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,
    save_dialog: Option<SaveChangesDialog>,
    users: UserBase,
    /// The published transaction this copy was taken from.
    revision: u64,
    /// Private draft saves do not advance the board revision.
    draft_changed: Arc<AtomicBool>,
    #[cfg(test)]
    rebuild_count: usize,
    added: Vec<Arc<Mutex<User>>>,
    removed: Vec<User>,
    has_changes: bool,
    /// Positions in the user base, in the order they are shown.
    view: Vec<usize>,
    search: String,
    searching: bool,
    sort: SortOrder,
}

impl UserList {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let user_len = icy_board.lock().unwrap().users.len();
        let mut list = Self {
            scroll_state: ScrollbarState::default().content_length(user_len),
            table_state: TableState::default().with_selected(if user_len > 0 { 0 } else { usize::MAX }),
            icy_board,
            users: UserBase::default(),
            revision: 0,
            draft_changed: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            rebuild_count: 0,
            added: Vec::new(),
            removed: Vec::new(),
            save_dialog: None,
            has_changes: false,
            view: Vec::new(),
            search: String::new(),
            searching: false,
            sort: SortOrder::Record,
        };
        list.rebuild_view();
        list
    }

    fn rebuild_view(&mut self) {
        self.draft_changed.swap(false, Ordering::Acquire);
        #[cfg(test)]
        {
            self.rebuild_count += 1;
        }
        let board = self.icy_board.lock().unwrap();
        self.revision = board.user_revision;
        self.users = board.users.clone();
        self.users.retain(|user| !self.removed.iter().any(|removed| same_identity(user, removed)));
        for user in &self.added {
            self.users.new_user(user.lock().unwrap().clone());
        }
        drop(board);
        let needle = self.search.trim().to_lowercase();
        let mut view: Vec<usize> = self
            .users
            .iter()
            .enumerate()
            .filter(|(_, user)| needle.is_empty() || user.get_name().to_lowercase().contains(&needle) || user.alias.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect();

        match self.sort {
            SortOrder::Record => {}
            SortOrder::Name => view.sort_by_key(|i| self.users[*i].get_name().to_lowercase()),
            SortOrder::Security => view.sort_by(|a, b| self.users[*b].security_level.cmp(&self.users[*a].security_level)),
            SortOrder::LastOn => view.sort_by(|a, b| self.users[*b].stats.last_on.cmp(&self.users[*a].stats.last_on)),
        }

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
            .map(|title| Cell::from(Text::from(Vec::from(config_title(title)))))
            .collect::<Row>()
            .height(2);

        let rows = self.view.iter().map(|i| {
            let user = &self.users[*i];
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
                Cell::from(format!("{:-3})", i + 1)),
                Cell::from(user.name.clone()),
                Cell::from(user.alias.clone()),
                Cell::from(user.security_level.to_string()),
                Cell::from(last_on),
                Cell::from(marker),
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
                Constraint::Min(15 + 1),
                Constraint::Length(3 + 1),
                Constraint::Length(10 + 1),
                Constraint::Length(2),
            ],
        )
        .header(header)
        .row_highlight_style(get_tui_theme().selected_item)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
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

        let mut new_idx = self.users.len() + 1;
        while self.users.iter().any(|user| user.name == format!("NewUser{new_idx}")) {
            new_idx += 1;
        }
        let new_user = User {
            name: format!("NewUser{new_idx}"),
            password: PasswordInfo {
                // A fresh record must not carry a password anyone could guess.
                password: Password::PlainText(format!("{:08x}{:08x}", fastrand::u32(..), fastrand::u32(..))),
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
        self.added.push(Arc::new(Mutex::new(new_user)));

        self.search.clear();
        self.sort = SortOrder::Record;
        self.rebuild_view();
        self.scroll_state = self.scroll_state.content_length(self.users.len());
        self.table_state.select(Some(self.view.len().saturating_sub(1)));
        self.has_changes = true;
    }

    fn remove(&mut self) -> PageMessage {
        if let Some(index) = self.selected_user() {
            if index == 0 {
                return PageMessage::InfoBox(InfoState::Warning, get_text("icbsm_record_one_protected"));
            }
            if index < self.users.len() {
                let existing = self.users.len() - self.added.len();
                if index >= existing {
                    self.added.remove(index - existing);
                } else {
                    self.removed.push(self.users[index].clone());
                }
                self.rebuild_view();
                self.has_changes = !self.added.is_empty() || !self.removed.is_empty();
            }
        }
        PageMessage::None
    }

    fn open_save_dialog(&mut self) {
        if self.save_dialog.is_none() {
            self.save_dialog = Some(SaveChangesDialog::new());
        }
    }

    #[cfg(test)]
    fn hotkeys(&self) -> HotkeyBar {
        self.hotkeys_with_sort(Some(false))
    }

    /// `None` omits the sort action, `Some(true)` shows only the current order.
    fn hotkeys_with_sort(&self, sort: Option<bool>) -> HotkeyBar {
        if self.searching {
            return HotkeyBar::for_id("icbsm_user_list_search");
        }
        let mut bar = HotkeyBar::for_id("icbsm_user_list_actions");
        // F2 is ignored by the handler until the list has unsaved changes.
        bar.entries.retain(|entry| self.has_changes || !entry.keys.contains(&KeyCode::F(2)));
        let Some(short) = sort else { return bar };
        let order = get_text(self.sort.label());
        let label = if short {
            order
        } else {
            format!("{} ({order})", icy_board_tui::get_text("hotkey_sort"))
        };
        bar.append(HotkeyBar::new([Hotkey::new(KeyCode::F(4), label)]))
    }

    /// Search and count stay context text; only the keys carry hint styling.
    /// Long translations drop the context, then the sort wording and finally
    /// the sort action, rather than being clipped on the border.
    fn footer(&self, width: usize) -> Line<'static> {
        for (context, sort) in [(true, Some(false)), (false, Some(false)), (false, Some(true))] {
            let line = self.footer_line(context, sort);
            if line.width() <= width {
                return line;
            }
        }
        self.footer_line(false, None)
    }

    fn footer_line(&self, context: bool, sort: Option<bool>) -> Line<'static> {
        let mut spans = Vec::new();
        if context && (self.searching || !self.search.is_empty()) {
            spans.push(Span::styled(
                format!(" {}: {}  ·  {} ", icy_board_tui::get_text("hotkey_search"), self.search, self.view.len()),
                get_tui_theme().description_text,
            ));
        }
        spans.extend(self.hotkeys_with_sort(sort).line().spans);
        Line::from(spans).centered()
    }

    /// Border-only context: never take a row away from the list or search.
    fn selection_summary(&self, available_width: usize) -> Option<Line<'static>> {
        let index = self.selected_user()?;
        let user = self.users.get(index)?;
        let mut text = format!(" #{} · {}", index + 1, user.get_name());
        if !user.city_or_state.is_empty() {
            text.push_str(&format!(" · {}", user.city_or_state));
        }
        text.push(' ');
        let line = Line::styled(text, get_tui_theme().description_text);
        (line.width() <= available_width).then_some(line.right_aligned())
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
        let result = self.icy_board.lock().unwrap().edit_users(|users| {
            for removed in &self.removed {
                let mut matches = users.iter().enumerate().filter(|(_, user)| user.name == removed.name);
                let (index, live) = matches.next().ok_or(UserUpdateError::MissingIdentity)?;
                if matches.next().is_some() {
                    return Err(UserUpdateError::AmbiguousIdentity.into());
                }
                if !same_identity(live, removed) {
                    return Err(UserUpdateError::IdentityChanged.into());
                }
                if index == 0 {
                    return Err(get_text("icbsm_record_one_protected").into());
                }
                if !super::user_editor::same_user_edit(live, removed) {
                    return Err(UserUpdateError::Conflict { field: "removed user".into() }.into());
                }
                users.remove(index);
            }
            for user in &self.added {
                let user = user.lock().unwrap().clone();
                if users.iter().any(|live| live.name == user.name) {
                    return Err(UserUpdateError::AmbiguousIdentity.into());
                }
                users.new_user(user);
            }
            Ok(())
        });
        match result {
            Ok(_) => {
                self.added.clear();
                self.removed.clear();
                self.has_changes = false;
                PageMessage::Close
            }
            Err(e) => {
                log::error!("Failed to save user database: {e}");
                PageMessage::InfoBox(
                    InfoState::Error,
                    get_text_args("icbsm_save_failed", std::collections::HashMap::from([("error".to_string(), e.to_string())])),
                )
            }
        }
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
        self.save_dialog.as_ref()?;
        let dlg = self.save_dialog.as_mut().unwrap();
        match dlg.handle_key_press(key) {
            SaveChangesMessage::Save => {
                self.save_dialog = None;
                Some(self.try_save())
            }
            SaveChangesMessage::Close => {
                self.save_dialog = None;
                self.has_changes = false;
                self.added.clear();
                self.removed.clear();
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
        // Refresh saved editor changes without publishing pending list operations.
        if self.draft_changed.load(Ordering::Acquire) || self.revision != self.icy_board.lock().unwrap().user_revision {
            self.rebuild_view();
        }
        let area = area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let title = dirty_title(get_text("icbsm_menu_edit_users"), self.has_changes);
        let summary_width = (area.width as usize).saturating_sub(Line::raw(title.clone()).width() + 4);
        let mut block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(frame_title(title, get_tui_theme().dialog_box_title));
        if let Some(summary) = self.selection_summary(summary_width) {
            block = block.title(summary);
        }
        if self.save_dialog.is_none() {
            block = block.title_bottom(self.footer(usize::from(area.width.saturating_sub(2))));
        }
        block.render(area, frame.buffer_mut());

        let inner = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.render_table(frame, inner);
        self.render_scrollbar(frame, inner);

        if let Some(dlg) = &mut self.save_dialog {
            let backdrop = frame.area();
            dim_background(frame.buffer_mut(), backdrop);
            dlg.render(frame, area);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        // If dialog is open, it owns the key events.
        // The dialog owns the key, so its answer is passed on unchanged and the
        // list never sees the same key a second time.
        if let Some(result) = self.handle_save_dialog_keys(key) {
            return result;
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
            KeyCode::Delete => self.remove(),
            KeyCode::F(2) if self.has_changes => self.try_save(),
            KeyCode::Enter => {
                if let Some(index) = self.selected_user() {
                    let existing = self.users.len() - self.added.len();
                    let editor = if index >= existing {
                        UserEditor::for_new_user(self.icy_board.clone(), index, self.added[index - existing].clone(), self.draft_changed.clone())
                    } else {
                        UserEditor::from_snapshot(self.icy_board.clone(), index, self.users[index].clone())
                    };
                    PageMessage::OpenSubPage(Box::new(editor))
                } else {
                    PageMessage::None
                }
            }
            _ => PageMessage::None,
        }
    }
}

fn same_identity(left: &User, right: &User) -> bool {
    left.name == right.name && left.stats.first_date_on == right.stats.first_date_on
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    use crate::tabs::user_save_tests::Fixture;
    use icy_board_engine::icy_board::user_base::User;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn list() -> UserList {
        let mut board = IcyBoard::default();
        for name in ["Alice", "Bob"] {
            board.users.new_user(User {
                name: name.into(),
                city_or_state: "Berlin".into(),
                ..Default::default()
            });
        }
        UserList::new(Arc::new(Mutex::new(board)))
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    fn edit_selected_name(list: &mut UserList, terminal: &mut Terminal<TestBackend>, name: &str) {
        let old_name = list.users[list.selected_user().unwrap()].name.clone();
        let PageMessage::OpenSubPage(mut editor) = list.handle_key_press(KeyCode::Enter.into()) else {
            panic!("selected user must open an editor");
        };
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        editor.handle_key_press(KeyCode::End.into());
        for _ in old_name.chars() {
            editor.handle_key_press(KeyCode::Backspace.into());
        }
        for ch in name.chars() {
            editor.handle_key_press(KeyCode::Char(ch).into());
        }
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        assert!((0..25).any(|y| row_text(terminal.backend().buffer(), y).contains(name)));
        assert!(matches!(editor.handle_key_press(KeyCode::Esc.into()), PageMessage::None));
        terminal.draw(|frame| editor.render(frame, frame.area())).unwrap();
        editor.handle_key_press(KeyCode::Left.into());
        assert!(matches!(editor.handle_key_press(KeyCode::Enter.into()), PageMessage::Close));
    }

    #[test]
    fn rendered_list_refreshes_saved_private_draft_once_without_publishing_pending_operations() {
        let fixture = Fixture::new();
        let before = fixture.snapshot();
        let revision = fixture.board.lock().unwrap().user_revision;
        let mut list = UserList::new(fixture.board.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        list.handle_key_press(KeyCode::Down.into());
        list.handle_key_press(KeyCode::Delete.into());
        list.handle_key_press(KeyCode::Insert.into());
        list.handle_key_press(KeyCode::F(3).into());
        list.handle_key_press(KeyCode::Char('r').into());
        list.handle_key_press(KeyCode::Enter.into());
        list.handle_key_press(KeyCode::F(4).into());
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        assert!(row_text(terminal.backend().buffer(), 4).contains("NewUser3"));
        let rebuilds = list.rebuild_count;

        edit_selected_name(&mut list, &mut terminal, "Aaron");
        fixture.assert_unchanged(&before);
        assert_eq!(fixture.board.lock().unwrap().user_revision, revision);
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        assert!(row_text(terminal.backend().buffer(), 4).contains("Aaron"));
        assert!(!row_text(terminal.backend().buffer(), 4).contains("NewUser3"));
        assert_eq!(list.rebuild_count, rebuilds + 1);
        assert_eq!(list.search, "r");
        assert!(list.sort == SortOrder::Name);
        assert_eq!(list.view.len(), 1);
        assert_eq!(list.added.len(), 1);
        assert_eq!(list.added[0].lock().unwrap().name, "Aaron");
        assert_eq!(list.removed.len(), 1);
        assert_eq!(list.removed[0].name, "Charlie");
        assert!(list.has_changes);
        for _ in 0..3 {
            terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
            assert!(row_text(terminal.backend().buffer(), 4).contains("Aaron"));
        }
        assert_eq!(list.rebuild_count, rebuilds + 1, "idle frames must not recopy users");
        fixture.assert_unchanged(&before);

        list.handle_key_press(KeyCode::F(3).into());
        list.handle_key_press(KeyCode::Enter.into());
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        for (y, name) in [(4, "Aaron"), (5, "Bob"), (6, "Sysop")] {
            assert!(row_text(terminal.backend().buffer(), y).contains(name));
        }
    }

    #[test]
    fn rendered_list_refreshes_saved_user_revision_once_and_preserves_pending_operations() {
        let fixture = Fixture::new();
        let revision = fixture.board.lock().unwrap().user_revision;
        let mut list = UserList::new(fixture.board.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        list.handle_key_press(KeyCode::Down.into());
        list.handle_key_press(KeyCode::Delete.into());
        list.handle_key_press(KeyCode::Insert.into());
        list.handle_key_press(KeyCode::F(4).into());
        list.handle_key_press(KeyCode::Home.into());
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        assert!(row_text(terminal.backend().buffer(), 4).contains("Bob"));
        let rebuilds = list.rebuild_count;

        edit_selected_name(&mut list, &mut terminal, "Zelda");
        assert!(fixture.board.lock().unwrap().user_revision > revision);
        let persisted = std::fs::read(fixture.dir.join("users.toml")).unwrap();
        let users = fixture.disk_users();
        assert_eq!(users.len(), 3);
        assert_eq!(users[1].name, "Charlie");
        assert_eq!(users[2].name, "Zelda");
        for _ in 0..3 {
            terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
            for (y, name) in [(4, "NewUser3"), (5, "Sysop"), (6, "Zelda")] {
                assert!(row_text(terminal.backend().buffer(), y).contains(name));
            }
        }
        assert_eq!(list.rebuild_count, rebuilds + 1, "only the revision change must recopy users");
        assert!(list.sort == SortOrder::Name);
        assert_eq!(list.added.len(), 1);
        assert_eq!(list.removed.len(), 1);
        assert!(list.has_changes);
        assert_eq!(std::fs::read(fixture.dir.join("users.toml")).unwrap(), persisted);
    }

    #[test]
    fn list_save_failure_keeps_insertions_and_removals_private_for_retry() {
        let fixture = Fixture::new();
        let mut list = UserList::new(fixture.board.clone());
        let before = fixture.snapshot();
        list.table_state.select(Some(1));
        list.remove();
        list.insert();
        fixture.assert_unchanged(&before);
        {
            let mut board = fixture.board.lock().unwrap();
            board.users[0].email = "pending-normalization@example.invalid".into();
            board.config.paths.user_file = fixture.dir.clone();
        }
        let before = fixture.snapshot();
        assert!(matches!(list.try_save(), PageMessage::InfoBox(InfoState::Error, _)));
        fixture.assert_unchanged(&before);
        assert!(list.has_changes);
        assert_eq!(list.added.len(), 1);
        assert_eq!(list.removed.len(), 1);

        fixture.board.lock().unwrap().config.paths.user_file = fixture.dir.join("users.toml");
        assert!(matches!(list.try_save(), PageMessage::Close));
        let users = fixture.disk_users();
        assert_eq!(users.len(), 3);
        assert_eq!(users[1].name, "Bob");
        assert!(users[2].name.starts_with("NewUser"));
        assert!(!list.has_changes);
    }

    #[test]
    fn list_discard_preserves_other_live_updates() {
        let mut fixture = Fixture::new();
        let mut list = UserList::new(fixture.board.clone());
        list.insert();
        fixture
            .board
            .lock()
            .unwrap()
            .edit_users(|users| {
                users[1].sysop_comment = "another editor".into();
                Ok(())
            })
            .unwrap();
        fixture.persisted = std::fs::read(fixture.dir.join("users.toml")).unwrap();
        let before = fixture.snapshot();
        list.open_save_dialog();
        assert!(matches!(list.handle_key_press(KeyCode::Enter.into()), PageMessage::Close));
        fixture.assert_unchanged(&before);
    }

    #[test]
    fn list_delete_conflict_does_not_publish_any_other_pending_change() {
        let mut fixture = Fixture::new();
        let mut list = UserList::new(fixture.board.clone());
        list.table_state.select(Some(1));
        list.remove();
        list.insert();
        fixture
            .board
            .lock()
            .unwrap()
            .edit_users(|users| {
                users[1].sysop_comment = "updated after deletion was staged".into();
                Ok(())
            })
            .unwrap();
        fixture.persisted = std::fs::read(fixture.dir.join("users.toml")).unwrap();
        let before = fixture.snapshot();
        assert!(matches!(list.try_save(), PageMessage::InfoBox(InfoState::Error, _)));
        fixture.assert_unchanged(&before);
    }

    #[test]
    fn classic_size_keeps_rows_search_and_theme_selection() {
        let mut list = list();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(row_text(buffer, 1).contains(&format!(" {} ", get_text("icbsm_menu_edit_users"))));
        assert!(row_text(buffer, 4).contains("Alice"));
        assert!(row_text(buffer, 5).contains("Bob"));
        // The first record stays on the original first data row; the summary
        // shares the top border rather than introducing another heading row.
        let theme = get_tui_theme();
        assert_eq!(buffer[(12, 4)].fg, theme.selected_item.fg.unwrap());
        assert_eq!(buffer[(12, 4)].bg, theme.selected_item.bg.unwrap());
        assert_eq!(buffer[(12, 5)].fg, theme.table.fg.unwrap());

        list.handle_key_press(KeyEvent::from(KeyCode::F(3)));
        list.handle_key_press(KeyEvent::from(KeyCode::Char('B')));
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(list.selected_user(), Some(1));
        assert!(row_text(buffer, 4).contains("Bob"));
        assert!(row_text(buffer, 23).contains(&list.footer(74).to_string()));
    }

    #[test]
    fn selection_summary_is_optional_and_measured_in_cells() {
        let mut list = list();
        let summary = list.selection_summary(80).unwrap();
        let text: String = summary.spans.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(text, " #1 · Alice · Berlin ");
        assert!(list.selection_summary(summary.width()).is_some());
        assert!(list.selection_summary(summary.width() - 1).is_none());
        list.search = "no match".into();
        list.rebuild_view();
        assert!(list.selection_summary(80).is_none());
    }

    #[test]
    fn save_modal_dims_the_painted_list_and_hides_its_shortcuts() {
        let mut list = list();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        let mut expected = terminal.backend().buffer().clone();
        let area = expected.area;
        dim_background(&mut expected, area);
        list.open_save_dialog();
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        let actual = terminal.backend().buffer();
        // Both data rows are outside the centered confirmation box.
        for y in [4, 5] {
            for x in 3..77 {
                assert_eq!(actual[(x, y)], expected[(x, y)]);
            }
        }
        assert!(!(18..24).any(|y| row_text(actual, y).contains("F3")));
    }

    #[test]
    fn actions_are_conditional_and_context_is_not_parsed_as_keys() {
        let mut list = list();
        for dirty in [false, true] {
            list.has_changes = dirty;
            for sort in [SortOrder::Record, SortOrder::Name, SortOrder::Security, SortOrder::LastOn] {
                list.sort = sort;
                let bar = list.hotkeys();
                assert_eq!(bar.entries.iter().any(|entry| entry.keys == [KeyCode::F(2)]), dirty);
                let sort_actions: Vec<_> = bar.entries.iter().filter(|entry| entry.keys == [KeyCode::F(4)]).collect();
                assert_eq!(sort_actions.len(), 1);
                assert!(sort_actions[0].label.contains(&get_text(sort.label())));
                let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
                terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
                let buffer = terminal.backend().buffer();
                assert!(row_text(buffer, 23).contains(&list.footer(74).to_string()));
                // Where F4 still fits, its label names the current order.
                let footer = list.footer(74).to_string();
                assert!(!footer.contains("F4") || footer.contains(&get_text(sort.label())), "{footer}");
                assert!(row_text(buffer, 4).contains("Alice"));
            }
        }
        list.search = "F2 Enter Esc".into();
        list.searching = true;
        list.rebuild_view();
        assert_eq!(list.hotkeys().entries, HotkeyBar::for_id("icbsm_user_list_search").entries);
        assert!(list.footer(74).to_string().contains("F2 Enter Esc"));
    }
}
