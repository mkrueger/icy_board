use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::user_base::UserBase;
use icy_board_tui::chrome::{dim_background, dirty_title};
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
    backup: UserBase,
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
        let backup = icy_board.lock().unwrap().users.clone();
        let mut list = Self {
            scroll_state: ScrollbarState::default().content_length(user_len),
            table_state: TableState::default().with_selected(if user_len > 0 { 0 } else { usize::MAX }),
            icy_board,
            backup,
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
            .map(|title| Cell::from(Text::from(Vec::from(config_title(title)))))
            .collect::<Row>()
            .height(2);

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

        let mut board = self.icy_board.lock().unwrap();
        let new_idx = board.users.len() + 1;
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

    fn remove(&mut self) -> PageMessage {
        if let Some(index) = self.selected_user() {
            if index == 0 {
                return PageMessage::InfoBox(InfoState::Warning, get_text("icbsm_record_one_protected"));
            }
            let mut board = self.icy_board.lock().unwrap();
            if index < board.users.len() {
                board.users.remove(index);
                drop(board);
                self.rebuild_view();
                self.has_changes = true;
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
        let board = self.icy_board.lock().unwrap();
        let user = board.users.get(index)?;
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
        match self.icy_board.lock().unwrap().save_userbase() {
            Ok(_) => {
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

        let title = dirty_title(get_text("icbsm_menu_edit_users"), self.has_changes);
        let summary_width = (area.width as usize).saturating_sub(Line::raw(title.clone()).width() + 4);
        let mut block = Block::new()
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(Span::styled(title, get_tui_theme().dialog_box_title));
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
                    PageMessage::OpenSubPage(Box::new(UserEditor::new(self.icy_board.clone(), index)))
                } else {
                    PageMessage::None
                }
            }
            _ => PageMessage::None,
        }
    }
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
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

    #[test]
    fn classic_size_keeps_rows_search_and_theme_selection() {
        let mut list = list();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| list.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();
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
