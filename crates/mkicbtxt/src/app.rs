use std::{collections::HashMap, path::PathBuf, time::Duration};

use chrono::{Local, Timelike};
use color_eyre::{Result, eyre::Context};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::icy_board::icb_text::{IcbTextFile, IcbTextStyle, TextEntry};
use icy_board_tui::{
    TerminalType,
    app::get_screen_size,
    chrome::{dim_background, dirty_title, status_line},
    get_text, get_text_args,
    hotkeys::HotkeyBar,
    pcb_line::get_styled_pcb_line,
    term::next_event,
    text_field::{TextField, TextfieldState},
    theme::{Theme, get_tui_theme},
};
#[cfg(test)]
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use strum_macros::{Display, FromRepr};

use crate::tabs::*;

pub struct App<'a> {
    mode: Mode,
    tab: TabPageType,
    orig: IcbTextFile,
    file: PathBuf,

    status_line: String,
    full_screen: bool,

    filter: String,
    filter_state: TextfieldState,

    edit_state: TextfieldState,
    edit_entry: TextEntry,

    record_tab: RecordTab<'a>,
    about_tab: AboutTab,

    pub save: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Command,
    Edit,
    Filter,
    Jump,
    RequestQuit,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, Display, FromRepr, PartialEq, Eq)]
enum TabPageType {
    #[default]
    Record,
    About,
}

impl TabPageType {
    pub fn iter() -> impl Iterator<Item = Self> {
        vec![TabPageType::Record, TabPageType::About].into_iter()
    }
}

#[cfg(test)]
mod localization_tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use icy_board_engine::icy_board::icb_text::DEFAULT_DISPLAY_TEXT;
    use ratatui::backend::TestBackend;

    fn render(app: &mut App<'_>, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| app.ui(frame, frame.area())).unwrap();
        terminal.backend().buffer().clone()
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    fn press(app: &mut App<'_>, code: KeyCode) {
        app.handle_key_press(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn title_bar_renders_localized_application_and_tabs() {
        let mut text = IcbTextFile::default();
        let app = App::new(&mut text, PathBuf::from("ICBTEXT"), false);
        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        app.render_title_bar(area, &mut buffer);
        let rendered: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
        assert!(rendered.contains(&format!("{} (ICBTEXT)", get_text("app_mkicbtxt"))), "{rendered}");
        assert!(rendered.contains(&get_text("icbtext_tab_record")), "{rendered}");
        assert!(rendered.ends_with(&format!(" {} ", get_text("icbtext_tab_about"))), "{rendered}");
        // The band must not sink into the records below it.
        let (band, _) = title_band(&get_tui_theme());
        assert_ne!(band.bg, get_tui_theme().background.bg);
        assert!((0..80).all(|x| buffer[(x, 0)].bg == band.bg.unwrap() || buffer[(x, 0)].bg == get_tui_theme().tabs_selected.bg.unwrap()));
    }

    #[test]
    fn every_palette_gives_the_title_its_own_background() {
        use icy_board_engine::icy_board::icb_config::PcbScreenColors;
        use icy_board_tui::theme::{POLISHED_THEME, Theme};

        for colors in [PcbScreenColors::DEFAULT_1, PcbScreenColors::DEFAULT_2, PcbScreenColors::BLACK_AND_WHITE] {
            let theme = Theme::from_pcboard(&PcbScreenColors { colors });
            let (band, title) = title_band(&theme);
            assert_ne!(band.bg, theme.background.bg, "{colors:?}");
            assert_ne!(title.fg, band.bg, "{colors:?}");
            assert_eq!(band.bg, title.bg, "{colors:?}");
        }
        let (band, title) = title_band(&POLISHED_THEME);
        assert_eq!(band, POLISHED_THEME.title_bar);
        assert_eq!(title, POLISHED_THEME.app_title);
        assert_ne!(band.bg, POLISHED_THEME.background.bg);
    }

    #[test]
    fn dirty_title_tracks_accepted_model_not_draft_or_filter() {
        let mut text = DEFAULT_DISPLAY_TEXT.clone();
        let theme = get_tui_theme();
        let mut app = App::new(&mut text, PathBuf::from("ICBTEXT"), false);
        assert_eq!(get_tui_theme().selected_item, theme.selected_item);
        assert!(!row(&render(&mut app, 80, 25), 0).contains(" *"));

        press(&mut app, KeyCode::Enter);
        app.edit_entry.text.push_str(" changed");
        assert!(!row(&render(&mut app, 80, 25), 0).contains(" *"));
        press(&mut app, KeyCode::Esc);
        assert!(!app.record_tab.is_dirty(&app.orig));

        press(&mut app, KeyCode::Enter);
        app.edit_entry.text.push_str(" changed");
        press(&mut app, KeyCode::Enter);
        assert!(row(&render(&mut app, 80, 25), 0).contains("(ICBTEXT) *"));
        press(&mut app, KeyCode::F(4));
        assert!(!row(&render(&mut app, 80, 25), 0).contains(" *"));

        // Style-only edits also differ from the loaded model.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::F(2));
        press(&mut app, KeyCode::Enter);
        assert!(row(&render(&mut app, 80, 25), 0).contains(" *"));
        press(&mut app, KeyCode::F(4));
        press(&mut app, KeyCode::F(2));
        render(&mut app, 80, 25);
        press(&mut app, KeyCode::Char('z'));
        assert!(!row(&render(&mut app, 80, 25), 0).contains(" *"));
    }

    #[test]
    fn restoring_builtin_text_is_dirty_when_loaded_text_was_custom() {
        let mut text = DEFAULT_DISPLAY_TEXT.clone();
        text.get_mut(1).unwrap().text.push_str(" custom");
        let mut app = App::new(&mut text, PathBuf::from("ICBTEXT"), false);
        assert!(!app.record_tab.is_dirty(&app.orig));
        press(&mut app, KeyCode::F(4));
        assert!(row(&render(&mut app, 80, 25), 0).contains(" *"));
        let loaded = app.orig.get(1).unwrap().clone();
        *app.record_tab.get_selected_entry_mut().unwrap() = loaded;
        assert!(!row(&render(&mut app, 80, 25), 0).contains(" *"));
    }

    #[test]
    fn dos_layout_keeps_active_keys_and_labeled_previews_visible() {
        let mut text = DEFAULT_DISPLAY_TEXT.clone();
        let mut app = App::new(&mut text, PathBuf::from("ICBTEXT"), false);
        for (mode, preset) in [
            (Mode::Command, "mkicbtxt_command_keys"),
            (Mode::Edit, "mkicbtxt_edit_keys"),
            (Mode::Filter, "mkicbtxt_filter_keys"),
            (Mode::Jump, "mkicbtxt_jump_keys"),
            (Mode::RequestQuit, "mkicbtxt_quit_keys"),
        ] {
            app.mode = mode;
            assert_eq!(app.hotkeys().entries, HotkeyBar::for_id(preset).entries);
            for width in [80, 40] {
                let buffer = render(&mut app, width, 25);
                let bar = app.hotkeys();
                let rows = bar.rows(width);
                let first = 24 - bar.height(width);
                for (offset, expected) in rows.iter().enumerate() {
                    let actual = row(&buffer, first + offset as u16);
                    assert_eq!(actual.trim(), expected.to_string().trim(), "{mode:?}: {actual}");
                    assert_eq!(expected.alignment, Some(Alignment::Center));
                    assert!(expected.width() <= width as usize);
                }
                // Every action label survives wrapping, rather than falling back to bare keys.
                let footer = (first..24).map(|y| row(&buffer, y)).join(" ");
                for entry in bar.entries {
                    assert!(footer.contains(&entry.label), "{mode:?}: {footer}");
                }
                if mode == Mode::Edit {
                    let screen = (0..first).map(|y| row(&buffer, y)).join("\n");
                    for label in [
                        "icbtext_edit_original_text_title",
                        "icbtext_edit_preview_text_title",
                        "icbtext_edit_edit_text_title",
                    ] {
                        assert!(screen.contains(&get_text(label)), "{screen}");
                    }
                }
            }
        }
    }

    #[test]
    fn tiny_screens_are_safe_and_status_keeps_context_before_clock() {
        let mut text = DEFAULT_DISPLAY_TEXT.clone();
        let mut app = App::new(&mut text, PathBuf::from("ICBTEXT"), false);
        for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (23, 12), (40, 10), (80, 25)] {
            for mode in [Mode::Command, Mode::Edit, Mode::Filter, Mode::Jump, Mode::RequestQuit] {
                app.mode = mode;
                let buffer = render(&mut app, width, height);
                assert_eq!(buffer.area, Rect::new(0, 0, width, height));
            }
        }
        app.mode = Mode::Command;
        let buffer = render(&mut app, 20, 5);
        assert!(row(&buffer, 4).trim_start().starts_with("1/"));
        assert!(app.hotkeys().height(20) > 1);
        let first = 4 - app.hotkeys().height(20).min(4);
        assert_eq!(row(&buffer, first).trim(), app.hotkeys().rows(20)[0].to_string().trim());
    }

    #[test]
    fn measured_footer_preserves_filter_context_status_and_live_filter_semantics() {
        let mut text = DEFAULT_DISPLAY_TEXT.clone();
        let mut app = App::new(&mut text, PathBuf::from("ICBTEXT"), false);
        press(&mut app, KeyCode::F(2));
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.filter, "a");
        let buffer = render(&mut app, 80, 25);
        assert!(row(&buffer, 1).contains(&get_text_args("icbtext_filter_text", HashMap::from([("filter".into(), "a".into())]))));
        assert!(row(&buffer, 24).contains(&app.record_tab.request_status().status_line));
        press(&mut app, KeyCode::F(3));
        assert!(!app.hotkeys().entries.iter().any(|entry| entry.keys.contains(&KeyCode::F(2))));
    }
}

#[derive(Default)]
pub struct ResultState {
    pub _cursor: Option<(u16, u16)>,
    pub status_line: String,
}

impl<'a> App<'a> {
    pub fn new(icb_txt: &'a mut IcbTextFile, file: PathBuf, full_screen: bool) -> Self {
        let orig = icb_txt.clone();
        Self {
            orig,
            full_screen,
            file,
            record_tab: RecordTab::new(icb_txt),
            mode: Mode::default(),
            tab: TabPageType::Record,
            about_tab: AboutTab::default(),
            status_line: String::new(),
            filter: String::new(),
            filter_state: TextfieldState::default(),

            edit_entry: TextEntry::default(),
            edit_state: TextfieldState::default(),
            save: false,
        }
    }

    /// Run the app until the user quits.
    pub fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        self.update_state();
        while self.is_running() {
            self.draw(terminal)?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.mode != Mode::Quit
    }

    /// Draw a single frame of the app.
    fn draw(&mut self, terminal: &mut TerminalType) -> Result<()> {
        terminal
            .draw(|frame| {
                let screen = get_screen_size(frame, self.full_screen);
                self.ui(frame, screen);
                match self.mode {
                    Mode::Edit if screen.height.saturating_sub(1 + self.hotkeys().height(screen.width)) >= 12 && screen.width >= 24 => {
                        self.edit_state.set_cursor_position(frame)
                    }
                    Mode::Jump => self.edit_state.set_cursor_position(frame),
                    Mode::Filter => self.filter_state.set_cursor_position(frame),
                    _ => {}
                }
            })
            .wrap_err("terminal.draw")?;
        Ok(())
    }

    /// Handle events from the terminal.
    ///
    /// This function is called once per frame, The events are polled from the stdin with timeout of
    /// 1/50th of a second. This was chosen to try to match the default frame rate of a GIF in VHS.
    fn handle_events(&mut self) -> Result<()> {
        let timeout = Duration::from_secs_f64(1.0);
        match next_event(timeout)? {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => self.handle_key_press(key),
            _ => {}
        }
        Ok(())
    }

    fn get_tab(&self) -> &dyn TabPage {
        match self.tab {
            TabPageType::Record => &self.record_tab,
            TabPageType::About => &self.about_tab,
        }
    }

    fn get_tab_mut(&mut self) -> &mut dyn TabPage {
        match self.tab {
            TabPageType::Record => &mut self.record_tab,
            TabPageType::About => &mut self.about_tab,
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        use KeyCode::*;
        match self.mode {
            Mode::Edit => {
                match key.code {
                    Esc => self.mode = Mode::Command,
                    F(2) => {
                        self.edit_entry.style = self.edit_entry.style.next();
                    }
                    F(3) => {
                        self.edit_entry.style = self.edit_entry.style.prev();
                    }
                    F(4) => {
                        if let Some(entry) = self.record_tab.get_original_entry() {
                            self.edit_entry = entry.clone();
                        }
                    }
                    Enter => {
                        if let Some(edit) = self.record_tab.get_selected_entry_mut() {
                            *edit = self.edit_entry.clone();
                        }
                        self.mode = Mode::Command;
                    }
                    _ => {
                        self.edit_state.handle_input(key, &mut self.edit_entry.text);
                    }
                };
            }
            Mode::Filter => {
                match key.code {
                    Enter | Esc => self.mode = Mode::Command,

                    _ => {
                        self.filter_state.handle_input(key, &mut self.filter);
                        self.record_tab.set_filter(&self.filter);
                    }
                };
            }

            Mode::Jump => {
                match key.code {
                    Esc => self.mode = Mode::Command,

                    Enter => {
                        if let Ok(number) = self.edit_entry.text.parse::<usize>()
                            && number > 0
                        {
                            self.record_tab.set_filter("");
                            self.record_tab.jump(number - 1);
                            self.update_state();
                        }
                        self.mode = Mode::Command;
                    }

                    _ => {
                        self.edit_state.handle_input(key, &mut self.edit_entry.text);
                    }
                };
            }
            Mode::RequestQuit => {
                match key.code {
                    Left | Right => self.save = !self.save,
                    Enter => {
                        self.mode = Mode::Quit;
                    }
                    Esc => {
                        self.mode = Mode::Command;
                    }
                    _ => {}
                };
            }
            _ => {
                if self.get_tab().grab_focus() {
                    let state = self.get_tab_mut().handle_key_press(key);
                    self.status_line = state.status_line;
                    return;
                }

                match key.code {
                    Char('q') | Esc => {
                        if self.record_tab.is_dirty(&self.orig) {
                            self.mode = Mode::RequestQuit;
                        } else {
                            self.mode = Mode::Quit;
                        }
                    }
                    Char('h') | Left => self.prev_tab(),
                    Char('l') | Right => self.next_tab(),
                    F(2) => self.mode = Mode::Filter,
                    F(3) => {
                        self.edit_entry.text = "".to_string();
                        self.edit_state = TextfieldState::default().with_max_len(4).with_position(0).with_mask("0123456789".to_string());
                        self.mode = Mode::Jump;
                    }
                    F(4) => {
                        if let Some(orig_entry) = self.record_tab.get_original_entry().cloned()
                            && let Some(entry) = self.record_tab.get_selected_entry_mut()
                        {
                            *entry = orig_entry;
                        }
                    }
                    Char('d') | Enter => {
                        if let Some(edit) = self.record_tab.get_selected_entry_mut() {
                            self.edit_entry = edit.clone();
                            self.edit_state = TextfieldState::default().with_position(edit.text.len() as u16);
                            self.mode = Mode::Edit;
                        }
                    }

                    _ => {
                        let state = self.get_tab_mut().handle_key_press(key);
                        self.status_line = state.status_line;
                    }
                };
            }
        }
    }

    fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.update_state();
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.update_state();
    }

    fn ui(&mut self, frame: &mut Frame, area: Rect) {
        Block::new().style(get_tui_theme().background).render(area, frame.buffer_mut());
        let [body, status_line] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
        let hotkeys = self.hotkeys();
        let key_height = hotkeys.height(body.width).min(body.height);
        let content = Rect::new(body.x, body.y, body.width, body.height - key_height);
        let key_bar = Rect::new(body.x, content.bottom(), body.width, key_height);
        let [title_bar, mut tab] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(content);
        self.render_title_bar(title_bar, frame.buffer_mut());

        if !self.filter.is_empty() && tab.height > 0 {
            let filter_area = Rect::new(tab.x, tab.y, tab.width, 1);
            self.render_filter_text(filter_area, frame.buffer_mut());
            tab.y += 1;
            tab.height -= 1;
        }

        self.render_selected_tab(frame, tab);

        match self.mode {
            // The dialog fills twelve lines and cannot be folded; below that the list stays.
            Mode::Edit if content.height < 12 || content.width < 24 => {}
            Mode::Edit => {
                let edit_height = 12;
                let edit_area = centered(content, content.width.saturating_sub(3), edit_height);

                dim_background(frame.buffer_mut(), area);
                Clear.render(edit_area, frame.buffer_mut());
                let edit_title = get_text_args(
                    "icbtext_edit_title",
                    HashMap::from([("number".to_string(), self.record_tab.selected_record().to_string())]),
                );

                let record_length = get_text_args(
                    "icbtext_edit_record_length_title",
                    HashMap::from([("number".to_string(), self.edit_entry.text.len().to_string())]),
                );

                let justify = match self.edit_entry.justification {
                    icy_board_engine::icy_board::icb_text::IcbTextJustification::Left => get_text("icbtext_edit_justify_left"),
                    icy_board_engine::icy_board::icb_text::IcbTextJustification::Right => get_text("icbtext_edit_justify_right"),
                    icy_board_engine::icy_board::icb_text::IcbTextJustification::Center => get_text("icbtext_edit_justify_center"),
                };
                let justify_title = get_text_args("icbtext_edit_justify_title", HashMap::from([("justify".to_string(), justify)]));

                Block::new()
                    .borders(Borders::ALL)
                    .title(Line::from(Span::from(format!(" {} ", edit_title)).style(get_tui_theme().dialog_box_title)))
                    .title_position(TitlePosition::Bottom)
                    .title_alignment(Alignment::Center)
                    .title(Line::from(Span::from(format!(" {} ", record_length)).style(get_tui_theme().dialog_box_title)))
                    .title_position(TitlePosition::Top)
                    .title_alignment(Alignment::Right)
                    .title(Line::from(Span::from(format!(" {} ", justify_title)).style(get_tui_theme().dialog_box_title)))
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(edit_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.edit_entry.text.to_string());

                let mut area = edit_area.inner(Margin { horizontal: 1, vertical: 1 });
                area.height = 1;

                Line::from(get_text("icbtext_edit_original_text_title"))
                    .style(get_tui_theme().menu_label)
                    .render(area, frame.buffer_mut());

                if let Some(entry) = self.record_tab.get_original_entry() {
                    let mut style_area = area;
                    let indent = 30.min(style_area.width);
                    style_area.x += indent;
                    style_area.width -= indent;

                    Line::from(vec![
                        Span::styled(get_text("icbtext_edit_style"), get_tui_theme().dialog_box_title),
                        Span::raw(" "),
                        Span::styled(Self::get_style_description(entry.style), convert_style(entry.style).not_italic().bold()),
                        Span::raw(" "),
                    ])
                    .alignment(Alignment::Right)
                    .render(style_area, frame.buffer_mut());
                    area.y += 1;

                    Text::from(get_styled_pcb_line(&entry.text))
                        .style(convert_style(entry.style))
                        .render(area, frame.buffer_mut());
                }
                area.y += 2;
                Line::from(get_text("icbtext_edit_preview_text_title"))
                    .style(get_tui_theme().menu_label)
                    .render(area, frame.buffer_mut());
                area.y += 1;
                Text::from(get_styled_pcb_line(&self.edit_entry.text))
                    .style(convert_style(self.edit_entry.style))
                    .render(area, frame.buffer_mut());
                area.y += 2;

                Line::from(get_text("icbtext_edit_edit_text_title"))
                    .style(get_tui_theme().config_title.bold())
                    .render(area, frame.buffer_mut());

                let mut style_area = area;
                let indent = 30.min(style_area.width);
                style_area.x += indent;
                style_area.width -= indent;

                Line::from(vec![
                    Span::styled(get_text("icbtext_edit_style"), get_tui_theme().dialog_box_title),
                    Span::raw(" "),
                    Span::styled(
                        Self::get_style_description(self.edit_entry.style),
                        convert_style(self.edit_entry.style).not_italic().bold(),
                    ),
                    Span::raw(" "),
                ])
                .alignment(Alignment::Right)
                .render(style_area, frame.buffer_mut());
                area.y += 1;

                frame.render_stateful_widget(field, area, &mut self.edit_state);

                area.y += 2;
                Line::from(get_text("icbtext_edit_hard_space_info"))
                    .style(get_tui_theme().description_text)
                    .alignment(Alignment::Center)
                    .render(area, frame.buffer_mut());
            }
            Mode::Filter => {
                let filter_area = centered(content, content.width.saturating_sub(5), 3);

                dim_background(frame.buffer_mut(), area);
                Clear.render(filter_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Line::from(
                        Span::from(format!(" {} ", get_text("icbtext_filter_title"))).style(get_tui_theme().dialog_box_title),
                    ))
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(filter_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.filter.to_string());

                let area = filter_area.inner(Margin { horizontal: 2, vertical: 1 });
                frame.render_stateful_widget(field, area, &mut self.filter_state);
            }
            Mode::Jump => {
                let jump_size = 31;
                let jump_area = centered(content, jump_size, 3);

                dim_background(frame.buffer_mut(), area);
                Clear.render(jump_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Line::from(
                        Span::from(format!(" {} ", get_text("icbtext_jump_to_title"))).style(get_tui_theme().dialog_box_title),
                    ))
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(jump_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.edit_entry.text.to_string());

                let area = jump_area.inner(Margin { horizontal: 2, vertical: 1 });
                frame.render_stateful_widget(field, area, &mut self.edit_state);
            }
            Mode::RequestQuit => {
                let save_text = format!("{} ", get_text("icbtext_save_changes"));
                let save_area = centered(
                    content,
                    Line::from(save_text.as_str()).width().saturating_add(10).min(u16::MAX as usize) as u16,
                    3,
                );

                dim_background(frame.buffer_mut(), area);
                Clear.render(save_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(save_area, frame.buffer_mut());

                let field = Line::from(vec![
                    Span::styled(save_text, get_tui_theme().menu_label),
                    Span::styled(get_text("yes"), if self.save { get_tui_theme().selected_item } else { get_tui_theme().item }),
                    Span::styled("/", get_tui_theme().menu_label),
                    Span::styled(get_text("no"), if !self.save { get_tui_theme().selected_item } else { get_tui_theme().item }),
                ]);
                field.render(save_area.inner(Margin { horizontal: 1, vertical: 1 }), frame.buffer_mut());
            }
            _ => {}
        }
        // Keep active controls legible above the dimmed background, even on a tiny terminal.
        hotkeys.render(key_bar, frame.buffer_mut());
        self.render_status_line(status_line, frame.buffer_mut());
    }

    fn get_style_description(style: IcbTextStyle) -> String {
        match style {
            IcbTextStyle::Plain => get_text("icbtext_style_plain"),
            IcbTextStyle::Red => get_text("icbtext_style_red"),
            IcbTextStyle::Green => get_text("icbtext_style_green"),
            IcbTextStyle::Yellow => get_text("icbtext_style_yellow"),
            IcbTextStyle::Blue => get_text("icbtext_style_blue"),
            IcbTextStyle::Purple => get_text("icbtext_style_purple"),
            IcbTextStyle::Cyan => get_text("icbtext_style_cyan"),
            IcbTextStyle::White => get_text("icbtext_style_white"),
        }
    }

    fn update_state(&mut self) {
        let state = self.get_tab().request_status();
        self.status_line = state.status_line;
    }

    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let (band, title_style) = title_band(&get_tui_theme());
        Block::new().style(band).render(area, buf);
        let len: u16 = TabPageType::iter().map(|p| Line::from(p.title()).width() as u16).sum();
        let layout = Layout::horizontal([Constraint::Min(0), Constraint::Length(len)]);
        let [title, tabs] = layout.areas(area);

        Span::styled(
            dirty_title(
                format!(
                    " {}",
                    get_text_args(
                        "app_file_title",
                        HashMap::from([
                            ("application".to_string(), get_text("app_mkicbtxt")),
                            ("path".to_string(), self.file.file_name().unwrap_or_default().to_string_lossy().into_owned()),
                        ]),
                    )
                ),
                self.record_tab.is_dirty(&self.orig),
            ),
            title_style,
        )
        .render(title, buf);
        let titles = TabPageType::iter().map(TabPageType::title);
        Tabs::new(titles)
            .style(band)
            .highlight_style(get_tui_theme().tabs_selected)
            .select(self.tab as usize)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn render_filter_text(&self, area: Rect, buf: &mut Buffer) {
        Line::from(get_text_args(
            "icbtext_filter_text",
            HashMap::from([("filter".to_string(), self.filter.to_string())]),
        ))
        .style(get_tui_theme().filter_text.bold())
        .render(area, buf);
    }

    fn render_selected_tab(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        Block::new().style(get_tui_theme().background).render(area, frame.buffer_mut());
        self.get_tab_mut().render(frame, area);
    }

    fn hotkeys(&self) -> HotkeyBar {
        HotkeyBar::for_id(match self.mode {
            Mode::RequestQuit => "mkicbtxt_quit_keys",
            Mode::Filter => "mkicbtxt_filter_keys",
            Mode::Jump => "mkicbtxt_jump_keys",
            Mode::Edit => "mkicbtxt_edit_keys",
            _ => "mkicbtxt_command_keys",
        })
    }

    fn render_status_line(&self, area: Rect, buf: &mut Buffer) {
        let now = Local::now();
        let time_status = format!("{} {}", now.time().with_nanosecond(0).unwrap(), now.date_naive().format("%m-%d-%y"));
        let context = match self.mode {
            Mode::RequestQuit => get_text("icbtext_save_changes"),
            Mode::Filter => get_text("icbtext_filter_title"),
            Mode::Jump => get_text("icbtext_jump_to_title"),
            Mode::Edit => get_text_args(
                "icbtext_edit_title",
                HashMap::from([("number".to_string(), self.record_tab.selected_record().to_string())]),
            ),
            _ if self.tab == TabPageType::Record => self.record_tab.request_status().status_line,
            _ => self.status_line.clone(),
        };
        status_line(buf, area, &context, &time_status);
    }
}

impl TabPageType {
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    fn prev(self) -> Self {
        let current_index = self as usize;
        let prev_index = current_index.saturating_sub(1);
        Self::from_repr(prev_index).unwrap_or(self)
    }

    fn title(self) -> String {
        let t = match self {
            Self::Record => get_text("icbtext_tab_record"),
            Self::About => get_text("icbtext_tab_about"),
        };
        format!(" {t} ")
    }
}

/// A box of that size in the middle of the area, never bigger than the area itself.
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
}

/// The records start right below the title, so it needs a band of its own.
/// PCBoard palettes paint the outer box in the page's own background.
fn title_band(theme: &Theme) -> (Style, Style) {
    if theme.title_bar.bg == theme.background.bg {
        (theme.key_binding_description, theme.key_binding)
    } else {
        (theme.title_bar, theme.app_title)
    }
}
