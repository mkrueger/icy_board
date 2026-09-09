use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    user_maintenance::{self, SortKey},
};
use icy_board_tui::{
    BORDER_SET,
    chrome::frame_title,
    config_menu::ResultState,
    get_text, get_text_args,
    hotkeys::HotkeyBar,
    icbsetupmenu::IcbSetupMenuUI,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};
use std::collections::HashMap;

/// A list of choices in its own box, the way the original nested its menus.
pub struct MenuPage {
    page: IcbSetupMenuUI,
    open: Box<dyn Fn(i32) -> Option<Box<dyn Page>>>,
}

impl MenuPage {
    pub fn new(title: String, items: Vec<MenuItem<i32>>, open: Box<dyn Fn(i32) -> Option<Box<dyn Page>>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(items))
                .with_center_title(title)
                .with_footer("icbsm_menu_keys"),
            open,
        }
    }
}

impl Page for MenuPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        self.page.render(frame, disp_area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == KeyCode::Esc {
            return PageMessage::Close;
        }
        let (_, selected) = self.page.handle_key_press(key);
        if let Some(id) = selected
            && let Some(page) = (self.open)(id)
        {
            return PageMessage::OpenSubPage(page);
        }
        PageMessage::None
    }
}

/// Builds the whole sort branch: options, the two field lists and the run.
pub fn sort_options_page(icy_board: Arc<Mutex<IcyBoard>>) -> MenuPage {
    MenuPage::new(
        get_text("icbsm_sort_options_title"),
        vec![
            MenuItem::new(0, 'A', get_text("icbsm_sort_single_title")),
            MenuItem::new(1, 'B', get_text("icbsm_sort_multiple_title")),
        ],
        Box::new(move |id| {
            let board = icy_board.clone();
            Some(Box::new(if id == 0 { single_field_page(board) } else { multiple_field_page(board) }))
        }),
    )
}

const SINGLE_FIELDS: [(char, &str, SortKey); 8] = [
    ('A', "icbsm_sort_name", SortKey::Name),
    ('B', "icbsm_sort_password", SortKey::Password),
    ('C', "icbsm_sort_bus_phone", SortKey::BusinessPhone),
    ('D', "icbsm_sort_home_phone", SortKey::HomePhone),
    ('E', "icbsm_sort_registration", SortKey::RegistrationExpiration),
    ('F', "icbsm_sort_comment1", SortKey::Comment1),
    ('G', "icbsm_sort_comment2", SortKey::Comment2),
    ('H', "icbsm_sort_city", SortKey::City),
];

const MULTI_FIELDS: [(char, &str, SortKey); 8] = [
    ('A', "icbsm_sort_security_name", SortKey::SecurityThenName),
    ('B', "icbsm_sort_times_on_name", SortKey::TimesOnThenName),
    ('C', "icbsm_sort_dnld_name", SortKey::FilesDownloadedThenName),
    ('D', "icbsm_sort_upld_name", SortKey::FilesUploadedThenName),
    ('E', "icbsm_sort_file_ratio_name", SortKey::FileRatioThenName),
    ('F', "icbsm_sort_dnld_bytes_name", SortKey::BytesDownloadedThenName),
    ('G', "icbsm_sort_upld_bytes_name", SortKey::BytesUploadedThenName),
    ('H', "icbsm_sort_byte_ratio_name", SortKey::BytesRatioThenName),
];

fn field_page(icy_board: Arc<Mutex<IcyBoard>>, title: &str, fields: &'static [(char, &str, SortKey); 8]) -> MenuPage {
    let items = fields
        .iter()
        .enumerate()
        .map(|(index, (ch, label, _))| MenuItem::new(index as i32, *ch, get_text(label)))
        .collect();
    MenuPage::new(
        get_text(title),
        items,
        Box::new(move |id| {
            let (_, label, key) = fields[id as usize];
            Some(Box::new(SortPage::new(icy_board.clone(), key, get_text(label))))
        }),
    )
}

fn single_field_page(icy_board: Arc<Mutex<IcyBoard>>) -> MenuPage {
    field_page(icy_board, "icbsm_sort_single_title", &SINGLE_FIELDS)
}

fn multiple_field_page(icy_board: Arc<Mutex<IcyBoard>>) -> MenuPage {
    field_page(icy_board, "icbsm_sort_multiple_title", &MULTI_FIELDS)
}

/// The last step before the file is rewritten: the reverse question the
/// original asked, then the run.
struct SortPage {
    icy_board: Arc<Mutex<IcyBoard>>,
    key: SortKey,
    field: String,
    reverse: bool,
    result: Option<String>,
}

impl SortPage {
    fn new(icy_board: Arc<Mutex<IcyBoard>>, key: SortKey, field: String) -> Self {
        Self {
            icy_board,
            key,
            field,
            reverse: false,
            result: None,
        }
    }

    fn run(&mut self) {
        let mut board = self.icy_board.lock().unwrap();
        let users_file = board.resolve_file(&board.config.paths.user_file);
        if let Err(err) = user_maintenance::create_backup(&users_file) {
            self.result = Some(get_text_args("icbsm_backup_failed", HashMap::from([("error".to_string(), err.to_string())])));
            return;
        }

        let save = board.edit_users(|users| Ok(user_maintenance::sort(users, self.key, self.reverse)));
        drop(board);

        self.result = Some(match save {
            Ok(report) => get_text_args("icbsm_sort_done", HashMap::from([("count".to_string(), report.changed.to_string())])),
            Err(err) => get_text_args("icbsm_save_failed", HashMap::from([("error".to_string(), err.to_string())])),
        });
    }
}

impl Page for SortPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let (lines, bottom) = if let Some(result) = &self.result {
            (vec![Line::from(result.clone())], "icbsm_done_keys")
        } else {
            (
                vec![
                    Line::from(get_text_args("icbsm_sort_field", HashMap::from([("field".to_string(), self.field.clone())]))),
                    Line::from(""),
                    Line::from(get_text_args(
                        "icbsm_sort_reverse",
                        HashMap::from([("value".to_string(), get_text(if self.reverse { "icbsm_yes" } else { "icbsm_no" }))]),
                    )),
                ],
                "icbsm_sort_keys",
            )
        };

        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 0))
            .title_alignment(Alignment::Center)
            .title(frame_title(get_text("icbsm_sort_run_title"), get_tui_theme().dialog_box_title))
            .title_bottom(HotkeyBar::for_id(bottom).line());

        Paragraph::new(Text::from(lines))
            .style(get_tui_theme().item)
            .wrap(Wrap { trim: false })
            .block(block)
            .render(area, frame.buffer_mut());
    }

    fn request_status(&self) -> ResultState {
        ResultState::default()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.result.is_some() {
            return PageMessage::Close;
        }
        match key.code {
            KeyCode::Esc => PageMessage::Close,
            KeyCode::Enter | KeyCode::PageDown | KeyCode::F(2) => {
                self.run();
                PageMessage::None
            }
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char(' ') => {
                self.reverse = !self.reverse;
                PageMessage::None
            }
            _ => PageMessage::None,
        }
    }
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    #[cfg(unix)]
    #[test]
    fn sort_save_failure_preserves_order_and_security_metadata() {
        let fixture = crate::tabs::user_save_tests::Fixture::new();
        let before = fixture.fail_serialization();
        let mut page = SortPage::new(fixture.board.clone(), SortKey::Name, "Name".into());
        page.run();
        assert!(page.result.as_ref().is_some_and(|error| error.contains("users.toml")));
        assert!(user_maintenance::has_backup(&fixture.dir.join("users.toml")));
        fixture.assert_unchanged(&before);
    }

    #[test]
    fn nested_menu_uses_setup_title_height_and_footer() {
        let mut page = MenuPage::new("User maintenance".into(), vec![MenuItem::new(0, 'A', "Edit users".into())], Box::new(|_| None));
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| page.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();

        assert!(!row_text(buffer, 1).contains("User maintenance"));
        assert!(row_text(buffer, 2).contains("User maintenance"));
        assert_eq!(row_text(buffer, 3).chars().filter(|ch| *ch == '─').count(), 76);
        assert!(row_text(buffer, 5).contains("Edit users"));
        assert!(row_text(buffer, 24).contains(&HotkeyBar::for_id("icbsm_menu_keys").line().to_string()));
    }
}
