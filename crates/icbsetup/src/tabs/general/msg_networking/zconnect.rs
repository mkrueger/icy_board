use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use icy_board_engine::icy_board::{
    IcyBoard,
    zconnect::{ZconnectArea, ZconnectLink},
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbsetupmenu::IcbSetupMenuUI,
    insert_table::{Column, InsertTable},
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ScrollbarState, TableState, Widget},
};

type Board = Arc<Mutex<IcyBoard>>;

/// ZCONNECT is a separate network, not an FTN transport.
pub struct ZconnectSettings {
    page: IcbSetupMenuUI,
    board: Board,
}

impl ZconnectSettings {
    pub fn new(board: Board) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("zconnect_general")).with_help(get_text("zconnect_general-help")),
                MenuItem::new(1, 'B', get_text("zconnect_links")).with_help(get_text("zconnect_links-help")),
            ]))
            .with_center_title(get_text("msg_networking_zconnect")),
            board,
        }
    }
}

impl Page for ZconnectSettings {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        self.page.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == KeyCode::Esc {
            return PageMessage::Close;
        }
        let (state, selected) = self.page.handle_key_press(key);
        match selected {
            Some(0) => PageMessage::OpenSubPage(Box::new(ZconnectForm::general(self.board.clone()))),
            Some(1) => PageMessage::OpenSubPage(Box::new(ZconnectList::new(self.board.clone(), ListKind::Links))),
            _ => PageMessage::ResultState(state),
        }
    }
}

fn ensure_config_file(board: &mut IcyBoard) {
    if board.config.paths.zconnect_file.as_os_str().is_empty() {
        board.config.paths.zconnect_file = "zconnect.toml".into();
    }
}

/// All mutations, including nested list edits, opt into saving the separate file.
/// Merely viewing these pages must not dirty the board.
fn entry(key: &str, value: ListValue, update: impl Fn(&mut IcyBoard, &ListValue) -> bool + 'static) -> ConfigEntry<Board> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), value)
            .with_label_width(22)
            .with_edit_width(40)
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_update_value(Box::new(move |board: &Board, value| {
                let mut board = board.lock().unwrap();
                if update(&mut board, value) {
                    ensure_config_file(&mut board);
                }
            })),
    )
}

fn text(value: &str) -> ListValue {
    ListValue::Text(255, TextFlags::None, value.to_string())
}

/// Return a bounded content viewport, leaving room for title and navigation.
fn panel(frame: &mut Frame, area: Rect, title: &str, help: &str) -> Rect {
    Clear.render(area, frame.buffer_mut());
    Block::new()
        .borders(Borders::ALL)
        .border_set(icy_board_tui::BORDER_SET)
        .title_alignment(Alignment::Center)
        .title(Span::styled(get_text(title), get_tui_theme().dialog_box_title))
        .title_bottom(Span::styled(get_text(help), get_tui_theme().key_binding))
        .style(get_tui_theme().dialog_box)
        .render(area, frame.buffer_mut());
    area.inner(Margin { horizontal: 2, vertical: 2 })
}

struct ZconnectForm {
    menu: ConfigMenu<Board>,
    state: ConfigMenuState,
    title: &'static str,
    link: Option<usize>,
    text_cursors: HashMap<usize, usize>,
}

impl ZconnectForm {
    fn new(board: Board, title: &'static str, entries: Vec<ConfigEntry<Board>>, link: Option<usize>) -> Self {
        Self {
            menu: ConfigMenu { obj: board, entry: entries },
            state: ConfigMenuState::default(),
            title,
            link,
            text_cursors: HashMap::new(),
        }
    }

    fn general(board: Board) -> Self {
        let lock = board.lock().unwrap();
        let cfg = &lock.zconnect;
        let entries = vec![
            entry("zconnect_enabled", ListValue::Bool(cfg.enabled), |b, v| {
                let ListValue::Bool(value) = v else { return false };
                b.zconnect.enabled = *value;
                true
            }),
            entry("zconnect_local_system", text(&cfg.local_system), |b, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                b.zconnect.local_system = value.clone();
                true
            }),
            entry("zconnect_local_user", text(&cfg.local_user), |b, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                b.zconnect.local_user = value.clone();
                true
            }),
            entry("zconnect_config_file", ListValue::Path(lock.config.paths.zconnect_file.clone()), |b, v| {
                let ListValue::Path(value) = v else { return false };
                b.config.paths.zconnect_file = value.clone();
                true
            }),
            entry("zconnect_inbound", ListValue::Path(cfg.inbound.clone()), |b, v| {
                let ListValue::Path(value) = v else { return false };
                b.zconnect.inbound = value.clone();
                true
            }),
            entry("zconnect_outbound", ListValue::Path(cfg.outbound.clone()), |b, v| {
                let ListValue::Path(value) = v else { return false };
                b.zconnect.outbound = value.clone();
                true
            }),
        ];
        drop(lock);
        Self::new(board, "zconnect_general", entries, None)
    }

    fn link(board: Board, index: usize) -> Option<Self> {
        let link = board.lock().unwrap().zconnect.links.get(index)?.clone();
        let entries = vec![
            link_entry(index, "zconnect_id", text(&link.id), |l, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                l.id = value.clone();
                true
            }),
            link_entry(index, "zconnect_host", text(&link.host), |l, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                l.host = value.clone();
                true
            }),
            link_entry(index, "zconnect_port", ListValue::U32(u32::from(link.port), 1, u32::from(u16::MAX)), |l, v| {
                let ListValue::U32(value, _, _) = v else { return false };
                let Ok(port) = u16::try_from(*value) else { return false };
                if port == 0 {
                    return false;
                }
                l.port = port;
                true
            }),
            link_entry(index, "zconnect_username", text(&link.username), |l, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                l.username = value.clone();
                true
            }),
            link_entry(
                index,
                "zconnect_password",
                ListValue::Text(255, TextFlags::Password, link.password.clone()),
                |l, v| {
                    let ListValue::Text(_, _, value) = v else { return false };
                    l.password = value.clone();
                    true
                },
            ),
            link_entry(
                index,
                "zconnect_login",
                ListValue::ComboBox(ComboBox {
                    is_edit_open: false,
                    values: ["zconnect", "janus", "direct"].into_iter().map(|v| ComboBoxValue::new(v, v)).collect(),
                    cur_value: ComboBoxValue::new(&link.login, &link.login),
                    selected_item: 0,
                    first_item: 0,
                }),
                |l, v| {
                    let ListValue::ComboBox(combo) = v else { return false };
                    let value = &combo.cur_value.value;
                    if !matches!(value.as_str(), "zconnect" | "janus" | "direct") {
                        return false;
                    }
                    // The shared combo control calls updates even when only rendering.
                    if l.login == *value {
                        return false;
                    }
                    l.login = value.clone();
                    true
                },
            ),
            link_entry(index, "zconnect_timeout", ListValue::U32(link.timeout_secs, 1, 3600), |l, v| {
                let ListValue::U32(value, _, _) = v else { return false };
                if !(1..=3600).contains(value) {
                    return false;
                }
                l.timeout_secs = *value;
                true
            }),
            link_entry(index, "zconnect_remote_system", text(&link.remote_system), |l, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                l.remote_system = value.clone();
                true
            }),
        ];
        Some(Self::new(board, "zconnect_link_editor", entries, Some(index)))
    }

    fn area(board: Board, link: usize, index: usize) -> Option<Self> {
        let area = board.lock().unwrap().zconnect.links.get(link)?.areas.get(index)?.clone();
        let entries = vec![
            area_entry(link, index, "zconnect_remote_board", text(&area.remote_board), |a, v| {
                let ListValue::Text(_, _, value) = v else { return false };
                a.remote_board = value.clone();
                true
            }),
            area_entry(link, index, "zconnect_local_area", ListValue::Path(area.local_area.clone()), |a, v| {
                let ListValue::Path(value) = v else { return false };
                a.local_area = value.clone();
                true
            }),
            area_entry(link, index, "zconnect_read_only", ListValue::Bool(area.read_only), |a, v| {
                let ListValue::Bool(value) = v else { return false };
                a.read_only = *value;
                true
            }),
        ];
        Some(Self::new(board, "zconnect_area_editor", entries, None))
    }

    // The shared text field indexes UTF-8 by byte and only masks inactive
    // passwords. Keep the workaround local to this page: edit by character,
    // then paint a masked, character-boundary-safe viewport after ConfigMenu.
    fn edit_text(&mut self, key: KeyEvent) -> bool {
        let board = self.menu.obj.clone();
        let Some(item) = self.menu.get_item_mut(self.state.selected) else {
            return false;
        };
        let (mut value, limit) = match &item.value {
            ListValue::Text(limit, _, value) => (value.clone(), usize::from(*limit)),
            ListValue::Path(value) => (value.to_string_lossy().into_owned(), 4096),
            _ => return false,
        };
        let count = value.chars().count();
        let cursor = self.text_cursors.entry(self.state.selected).or_default();
        *cursor = (*cursor).min(count);
        let byte = value.char_indices().nth(*cursor).map_or(value.len(), |(i, _)| i);
        let mut changed = false;
        match key.code {
            KeyCode::Left => *cursor = cursor.saturating_sub(1),
            KeyCode::Right => *cursor = (*cursor + 1).min(count),
            KeyCode::Home => *cursor = 0,
            KeyCode::End => *cursor = count,
            KeyCode::Backspace if *cursor > 0 => {
                let start = value.char_indices().nth(*cursor - 1).map_or(0, |(i, _)| i);
                value.replace_range(start..byte, "");
                *cursor -= 1;
                changed = true;
            }
            KeyCode::Delete if *cursor < count => {
                value.remove(byte);
                changed = true;
            }
            KeyCode::Char(ch) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) => {
                if !ch.is_control() && count < limit {
                    value.insert(byte, ch);
                    *cursor += 1;
                    changed = true;
                }
            }
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Insert => {}
            _ => return false,
        }
        if changed {
            match &mut item.value {
                ListValue::Text(_, _, text) => *text = value,
                ListValue::Path(path) => *path = value.into(),
                _ => unreachable!(),
            }
            if let Some(update) = &item.update_value {
                update(&board, &item.value);
            }
        }
        true
    }

    fn render_text(&self, frame: &mut Frame, area: Rect) {
        let Some(item) = self.menu.get_item(self.state.selected) else { return };
        let value = match &item.value {
            ListValue::Text(_, TextFlags::Password, value) => "*".repeat(value.chars().count()),
            ListValue::Text(_, _, value) => value.clone(),
            ListValue::Path(path) => path.to_string_lossy().into_owned(),
            _ => return,
        };
        let Some(row) = self.state.item_pos.get(&self.state.selected) else { return };
        if *row < self.state.first_row || *row - self.state.first_row >= area.height {
            return;
        }
        let field = Rect::new(area.x + 25, area.y + *row - self.state.first_row, area.width.saturating_sub(28), 1);
        if field.is_empty() {
            return;
        }
        let cursor = self.text_cursors.get(&self.state.selected).copied().unwrap_or(0).min(value.chars().count());
        let chars: Vec<char> = value.chars().collect();
        let mut first = cursor;
        let mut cursor_x = 0;
        while first > 0 {
            let width = Line::raw(chars[first - 1].to_string()).width();
            if cursor_x + width >= usize::from(field.width) {
                break;
            }
            cursor_x += width;
            first -= 1;
        }
        Clear.render(field, frame.buffer_mut());
        Block::new().style(get_tui_theme().text_field_background).render(field, frame.buffer_mut());
        Line::raw(chars[first..].iter().collect::<String>())
            .style(get_tui_theme().text_field_text)
            .render(field, frame.buffer_mut());
        frame.set_cursor_position((field.x + cursor_x as u16, field.y));
    }
}

fn link_entry(index: usize, key: &str, value: ListValue, update: impl Fn(&mut ZconnectLink, &ListValue) -> bool + 'static) -> ConfigEntry<Board> {
    entry(key, value, move |b, v| b.zconnect.links.get_mut(index).is_some_and(|link| update(link, v)))
}

fn area_entry(link: usize, index: usize, key: &str, value: ListValue, update: impl Fn(&mut ZconnectArea, &ListValue) -> bool + 'static) -> ConfigEntry<Board> {
    link_entry(link, key, value, move |l, v| l.areas.get_mut(index).is_some_and(|area| update(area, v)))
}

impl Page for ZconnectForm {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = panel(
            frame,
            area,
            self.title,
            if self.link.is_some() { "zconnect_link_keys" } else { "zconnect_form_keys" },
        );
        if area.width > 28 && area.height > 0 {
            // Reflect the default save path chosen by an earlier field edit,
            // without replacing the user's in-progress filename while typing.
            if self.title == "zconnect_general" && self.state.selected != 3 {
                let path = self.menu.obj.lock().unwrap().config.paths.zconnect_file.clone();
                if let Some(item) = self.menu.get_item_mut(3) {
                    item.value = ListValue::Path(path);
                }
            }
            if let Some(item) = self.menu.get_item_mut(self.state.selected)
                && matches!(item.value, ListValue::Text(..) | ListValue::Path(_))
            {
                item.text_field_state = Default::default();
            }
            self.menu.render(area, frame, &mut self.state);
            self.render_text(frame, area);
        }
    }

    fn request_status(&self) -> ResultState {
        ResultState::status_line(self.menu.current_status_line(&self.state))
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == KeyCode::F(2)
            && let Some(link) = self.link
        {
            return PageMessage::OpenSubPage(Box::new(ZconnectList::new(self.menu.obj.clone(), ListKind::Areas(link))));
        }
        if self.edit_text(key) {
            return PageMessage::ResultState(self.request_status());
        }
        let result = self.menu.handle_key_press(key, &mut self.state);
        if result.edit_msg == EditMessage::Close {
            PageMessage::Close
        } else {
            PageMessage::ResultState(result)
        }
    }
}

#[derive(Clone, Copy)]
enum ListKind {
    Links,
    Areas(usize),
}

struct ZconnectList {
    board: Board,
    kind: ListKind,
    table: InsertTable<'static>,
}

impl ZconnectList {
    fn new(board: Board, kind: ListKind) -> Self {
        let source = board.clone();
        let columns = match kind {
            ListKind::Links => vec![
                Column::new(get_text("zconnect_id")).with_width(18),
                Column::new(get_text("zconnect_host")).with_width(34),
                Column::new(get_text("zconnect_areas")),
            ],
            ListKind::Areas(_) => vec![
                Column::new(get_text("zconnect_remote_board")).with_width(22),
                Column::new(get_text("zconnect_local_area")).with_width(30),
                Column::new(get_text("zconnect_read_only")),
            ],
        };
        let mut page = Self {
            board,
            kind,
            table: InsertTable {
                scroll_state: ScrollbarState::default(),
                table_state: TableState::default(),
                columns,
                numbered: true,
                get_content: Box::new(move |_, i, column| {
                    let board = source.lock().unwrap();
                    let value = match kind {
                        ListKind::Links => board.zconnect.links.get(*i).map(|l| match column {
                            0 => l.id.clone(),
                            1 => format!("{}:{}", l.host, l.port),
                            _ => l.areas.len().to_string(),
                        }),
                        ListKind::Areas(link) => board.zconnect.links.get(link).and_then(|l| l.areas.get(*i)).map(|a| match column {
                            0 => a.remote_board.clone(),
                            1 => a.local_area.display().to_string(),
                            _ => get_text(if a.read_only { "zconnect_yes" } else { "zconnect_no" }),
                        }),
                    };
                    Line::from(value.unwrap_or_default())
                }),
                content_length: 0,
            },
        };
        page.sync_selection();
        page
    }

    fn sync_selection(&mut self) {
        let board = self.board.lock().unwrap();
        let len = match self.kind {
            ListKind::Links => board.zconnect.links.len(),
            ListKind::Areas(link) => board.zconnect.links.get(link).map_or(0, |l| l.areas.len()),
        };
        let selected = if len == 0 {
            None
        } else {
            Some(self.table.table_state.selected().unwrap_or(0).min(len - 1))
        };
        self.table.content_length = len;
        self.table.table_state.select(selected);
        self.table.scroll_state = ScrollbarState::default().content_length(len).position(selected.unwrap_or(0));
    }

    fn open_editor(&self) -> PageMessage {
        let Some(index) = self.table.table_state.selected() else {
            return PageMessage::None;
        };
        let form = match self.kind {
            ListKind::Links => ZconnectForm::link(self.board.clone(), index),
            ListKind::Areas(link) => ZconnectForm::area(self.board.clone(), link, index),
        };
        form.map_or(PageMessage::None, |form| PageMessage::OpenSubPage(Box::new(form)))
    }

    fn insert(&mut self) -> PageMessage {
        let mut board = self.board.lock().unwrap();
        let index = match self.kind {
            ListKind::Links => {
                board.zconnect.links.push(ZconnectLink::default());
                board.zconnect.links.len() - 1
            }
            ListKind::Areas(link) => {
                let Some(link) = board.zconnect.links.get_mut(link) else {
                    return PageMessage::None;
                };
                link.areas.push(ZconnectArea::default());
                link.areas.len() - 1
            }
        };
        ensure_config_file(&mut board);
        drop(board);
        self.table.table_state.select(Some(index));
        self.sync_selection();
        self.open_editor()
    }

    fn delete(&mut self) {
        let Some(index) = self.table.table_state.selected() else { return };
        let mut board = self.board.lock().unwrap();
        let removed = match self.kind {
            ListKind::Links if index < board.zconnect.links.len() => {
                board.zconnect.links.remove(index);
                true
            }
            ListKind::Areas(link) => {
                if let Some(link) = board.zconnect.links.get_mut(link)
                    && index < link.areas.len()
                {
                    link.areas.remove(index);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if removed {
            ensure_config_file(&mut board);
        }
        drop(board);
        self.sync_selection();
    }
}

impl Page for ZconnectList {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.sync_selection();
        let (title, help) = match self.kind {
            ListKind::Links => ("zconnect_links", "zconnect_links_keys"),
            ListKind::Areas(_) => ("zconnect_areas", "zconnect_areas_keys"),
        };
        let area = panel(frame, area, title, help);
        if area.width > 1 && area.height > 0 {
            self.table.render_table(frame, area);
        }
    }

    fn request_status(&self) -> ResultState {
        ResultState::status_line(get_text(match self.kind {
            ListKind::Links => "zconnect_links-status",
            ListKind::Areas(_) => "zconnect_areas-status",
        }))
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.sync_selection();
        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::Insert => return self.insert(),
            KeyCode::Delete => self.delete(),
            KeyCode::Enter => return self.open_editor(),
            KeyCode::F(1) => {
                return PageMessage::ResultState(ResultState {
                    edit_msg: EditMessage::DisplayHelp(get_text(match self.kind {
                        ListKind::Links => "zconnect_links-help",
                        ListKind::Areas(_) => "zconnect_areas-help",
                    })),
                    status_line: self.request_status().status_line,
                });
            }
            KeyCode::F(2) if matches!(self.kind, ListKind::Links) => {
                if let Some(link) = self.table.table_state.selected() {
                    return PageMessage::OpenSubPage(Box::new(Self::new(self.board.clone(), ListKind::Areas(link))));
                }
            }
            _ => {
                let _ = self.table.handle_key_press(key);
            }
        }
        PageMessage::ResultState(self.request_status())
    }
}

#[cfg(test)]
#[path = "zconnect_tests.rs"]
mod tests;
