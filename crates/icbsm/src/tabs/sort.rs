use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    user_maintenance::{self, MaintenanceReport, SortKey},
};
use icy_board_tui::{
    BORDER_SET,
    config_menu::ResultState,
    get_text, get_text_args,
    select_menu::{MenuItem, SelectMenu, SelectMenuState},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};
use std::collections::HashMap;

/// A list of choices in its own box, the way the original nested its menus.
pub struct MenuPage {
    title: String,
    menu: SelectMenu<i32>,
    state: SelectMenuState,
    open: Box<dyn Fn(i32) -> Option<Box<dyn Page>>>,
}

impl MenuPage {
    pub fn new(title: String, items: Vec<MenuItem<i32>>, open: Box<dyn Fn(i32) -> Option<Box<dyn Page>>>) -> Self {
        Self {
            title,
            menu: SelectMenu::new(items),
            state: SelectMenuState::default(),
            open,
        }
    }
}

impl Page for MenuPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().menu_box)
            .padding(Padding::new(2, 2, 1, 0))
            .title_alignment(Alignment::Center)
            .title(Span::styled(self.title.clone(), get_tui_theme().menu_box_title))
            .title_bottom(Span::styled(get_text("icbsm_menu_keys"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());

        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().menu_box,
        );

        let width = self.menu.preferred_width();
        let mut menu_area = area.inner(Margin {
            vertical: 0,
            horizontal: (area.width.saturating_sub(width)) / 2,
        });
        menu_area.y += 4;
        menu_area.height = menu_area.height.saturating_sub(5);
        self.menu.render(menu_area, frame, &mut self.state);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == KeyCode::Esc {
            return PageMessage::Close;
        }
        if let Some(id) = self.menu.handle_key_press(key, &mut self.state)
            && let Some(page) = (self.open)(*id)
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

        let original = board.users.clone();
        let report: MaintenanceReport = user_maintenance::sort(&mut board.users, self.key, self.reverse);
        let save = board.save_userbase();
        if save.is_err() {
            board.users = original;
        }
        drop(board);

        self.result = Some(match save {
            Ok(()) => get_text_args("icbsm_sort_done", HashMap::from([("count".to_string(), report.changed.to_string())])),
            Err(err) => get_text_args("icbsm_save_failed", HashMap::from([("error".to_string(), err.to_string())])),
        });
    }
}

impl Page for SortPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let (lines, bottom) = if let Some(result) = &self.result {
            (vec![Line::from(result.clone())], get_text("icbsm_done_keys"))
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
                get_text("icbsm_sort_keys"),
            )
        };

        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 0))
            .title_alignment(Alignment::Center)
            .title(Span::styled(get_text("icbsm_sort_run_title"), get_tui_theme().dialog_box_title))
            .title_bottom(Span::styled(bottom, get_tui_theme().key_binding));

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
