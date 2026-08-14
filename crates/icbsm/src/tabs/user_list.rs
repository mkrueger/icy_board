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

pub struct UserList {
    scroll_state: ScrollbarState,
    table_state: TableState,
    icy_board: Arc<Mutex<IcyBoard>>,
    save_dialog: Option<SaveChangesDialog>,
    backup: UserBase,
    has_changes: bool,
    in_edit_mode: bool,
}

impl UserList {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let user_len = icy_board.lock().unwrap().users.len();
        let backup = icy_board.lock().unwrap().users.clone();
        Self {
            scroll_state: ScrollbarState::default().content_length(user_len),
            table_state: TableState::default().with_selected(if user_len > 0 { 0 } else { usize::MAX }),
            icy_board,
            backup,
            save_dialog: None,
            has_changes: false,
            in_edit_mode: false,
        }
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
        let header = ["", "Name", "Alias", "Security"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(get_tui_theme().table_header)
            .height(1);

        let l = self.icy_board.lock().unwrap();
        let rows = l.users.iter().enumerate().map(|(i, user)| {
            Row::new(vec![
                Cell::from(format!("{:-3})", i + 1)).style(get_tui_theme().item),
                Cell::from(user.name.clone()).style(get_tui_theme().item),
                Cell::from(user.alias.clone()).style(get_tui_theme().item),
                Cell::from(user.security_level.to_string()).style(get_tui_theme().item),
            ])
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(25 + 1),
                Constraint::Min(3 + 1),
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
        if self.icy_board.lock().unwrap().users.is_empty() {
            return;
        }
        let max = self.icy_board.lock().unwrap().users.len();
        let i = match self.table_state.selected() {
            Some(0) | None => max - 1,
            Some(i) => i - 1,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn next(&mut self) {
        if self.icy_board.lock().unwrap().users.is_empty() {
            return;
        }
        let max = self.icy_board.lock().unwrap().users.len();
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

        self.scroll_state = self.scroll_state.content_length(len);
        self.table_state.select(Some(len - 1));
        self.has_changes = true;
    }

    fn remove(&mut self) {
        if let Some(sel) = self.table_state.selected() {
            let mut board = self.icy_board.lock().unwrap();
            if sel < board.users.len() {
                board.users.remove(sel);
                let len = board.users.len();
                drop(board);

                self.scroll_state = self.scroll_state.content_length(len);
                if len == 0 {
                    self.table_state.select(None);
                } else if sel >= len {
                    self.table_state.select(Some(len - 1));
                } else {
                    self.table_state.select(Some(sel));
                }
                self.has_changes = true;
            }
        }
    }

    fn open_save_dialog(&mut self) {
        if self.save_dialog.is_none() {
            self.save_dialog = Some(SaveChangesDialog::new());
        }
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
            .border_type(BorderType::Double);
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
                if let Some(sel) = self.table_state.selected() {
                    self.in_edit_mode = true;
                    PageMessage::OpenSubPage(Box::new(UserEditor::new(self.icy_board.clone(), sel)))
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
