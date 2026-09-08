//! Typed, theme-aware hotkey hints. This module does not change input dispatch,
//! and it neither adds nor moves hint rows: callers keep their own placement,
//! normally a block's bottom border line via [`HotkeyBar::line`].

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::theme::get_tui_theme;

pub mod presets;

/// One action, with one or more alternative actual keys sharing modifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub keys: Vec<KeyCode>,
    pub modifiers: KeyModifiers,
    pub label: String,
}

impl Hotkey {
    pub fn new(key: KeyCode, label: impl Into<String>) -> Self {
        Self::alternatives([key], label)
    }

    pub fn alternatives(keys: impl IntoIterator<Item = KeyCode>, label: impl Into<String>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
            modifiers: KeyModifiers::NONE,
            label: label.into(),
        }
    }

    pub fn modified(modifiers: KeyModifiers, key: KeyCode, label: impl Into<String>) -> Self {
        Self {
            modifiers,
            ..Self::new(key, label)
        }
    }

    /// Exact code/modifier matching for presses and repeats (never releases).
    /// BackTab is normalized to Shift+Tab. Character case is otherwise exact;
    /// this helper does not infer an application's case-insensitive shortcuts.
    pub fn matches(&self, event: KeyEvent) -> bool {
        event.kind != KeyEventKind::Release
            && self
                .keys
                .iter()
                .any(|&key| normalize_key(key, self.modifiers) == normalize_key(event.code, event.modifiers))
    }

    /// Centralized key symbols; alternative chords are separated with `/`.
    pub fn key_text(&self) -> String {
        self.keys.iter().map(|&key| chord_text(key, self.modifiers)).collect::<Vec<_>>().join("/")
    }

    fn tokens(&self, styles: Styles) -> Vec<Span<'static>> {
        let mut tokens = Vec::new();
        let key = self.key_text();
        if !key.is_empty() {
            tokens.push(Span::styled(key, styles.key));
        }
        // Normalize whitespace so explicit one-line and wrapped usage agree.
        tokens.extend(self.label.split_whitespace().map(|word| Span::styled(word.to_owned(), styles.label)));
        tokens
    }

    fn spans(&self, styles: Styles) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for token in self.tokens(styles) {
            if !spans.is_empty() {
                spans.push(Span::styled(" ", styles.label));
            }
            spans.push(token);
        }
        spans
    }
}

fn normalize_key(key: KeyCode, modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
    if key == KeyCode::BackTab {
        (KeyCode::Tab, modifiers | KeyModifiers::SHIFT)
    } else {
        (key, modifiers)
    }
}

fn chord_text(key: KeyCode, modifiers: KeyModifiers) -> String {
    let (key, modifiers) = normalize_key(key, modifiers);
    let mut text = String::new();
    for (modifier, name) in [
        (KeyModifiers::CONTROL, "Ctrl+"),
        (KeyModifiers::ALT, "Alt+"),
        (KeyModifiers::SHIFT, "Shift+"),
        (KeyModifiers::SUPER, "Super+"),
        (KeyModifiers::HYPER, "Hyper+"),
        (KeyModifiers::META, "Meta+"),
    ] {
        if modifiers.contains(modifier) {
            text.push_str(name);
        }
    }
    text.push_str(&key_symbol(key));
    text
}

/// Terminal-friendly symbols, without emoji presentation/variation selectors.
pub fn key_symbol(key: KeyCode) -> String {
    match key {
        KeyCode::Enter => "↵".into(),
        KeyCode::Esc => "␛".into(),
        KeyCode::Backspace => "⌫".into(),
        KeyCode::Tab => "⇥".into(),
        KeyCode::BackTab => "Shift+⇥".into(),
        KeyCode::Delete => "⌦".into(),
        KeyCode::Insert => "⎀".into(),
        KeyCode::Home => "↖".into(),
        KeyCode::End => "↘".into(),
        KeyCode::PageUp => "⇞".into(),
        KeyCode::PageDown => "⇟".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Char(' ') => "␠".into(),
        KeyCode::Char(c) if c.is_control() => format!("U+{:04X}", c as u32),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Null => "Null".into(),
        // Less common enhanced-keyboard/media keys retain their textual names.
        other => format!("{other:?}"),
    }
}

#[derive(Clone, Copy, Debug)]
struct Styles {
    key: Style,
    label: Style,
}

/// A composable footer whose measurement and rendering share the same layout.
#[derive(Clone, Debug, Default)]
pub struct HotkeyBar {
    pub entries: Vec<Hotkey>,
    styles: Option<Styles>,
}

impl HotkeyBar {
    pub fn new(entries: impl IntoIterator<Item = Hotkey>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
            styles: None,
        }
    }

    /// Look up structured presets, never parse a legacy localized hint string.
    pub fn for_id(id: &str) -> Self {
        presets::for_id(id)
    }

    /// Append entries in order. The receiving bar's style selection wins.
    pub fn append(mut self, other: HotkeyBar) -> Self {
        self.entries.extend(other.entries);
        self
    }

    /// Override the hint styles, e.g. with an explicitly selected test theme.
    /// Without this override the active theme is read at layout/render time.
    pub fn with_styles(mut self, key_style: Style, label_style: Style) -> Self {
        self.styles = Some(Styles {
            key: key_style,
            label: label_style,
        });
        self
    }

    fn styles(&self) -> Styles {
        self.styles.unwrap_or_else(|| {
            let theme = get_tui_theme();
            Styles {
                key: theme.key_binding,
                label: theme.key_binding_description,
            }
        })
    }

    /// One centered line, e.g. as a block's bottom border title. It is a single
    /// contiguous block and may exceed the caller's width.
    pub fn line(&self) -> Line<'static> {
        let styles = self.styles();
        let mut spans = Vec::new();
        for entry in &self.entries {
            let entry = entry.spans(styles);
            if entry.is_empty() {
                continue;
            }
            spans.push(Span::styled(if spans.is_empty() { " " } else { "  " }, styles.label));
            spans.extend(entry);
        }
        if !spans.is_empty() {
            spans.push(Span::styled(" ", styles.label));
        }
        Line::from(spans).centered()
    }

    /// Center each row independently, keeping entries intact whenever possible.
    /// Oversized entries wrap at words, then at extended grapheme boundaries;
    /// their key appears first. Width zero yields no rows. At width one a
    /// two-cell grapheme cannot physically fit and is displayed as `�` instead.
    /// No other text is truncated and no trailing blank rows are emitted.
    pub fn rows(&self, width: u16) -> Vec<Line<'static>> {
        self.layout(width, self.styles())
    }

    fn layout(&self, width: u16, styles: Styles) -> Vec<Line<'static>> {
        // Both padding cells belong to the coloured block, like a DOS hint bar.
        let Some(width) = usize::from(width).checked_sub(2).filter(|width| *width > 0) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        let mut row = Line::default();
        for entry in &self.entries {
            let entry_line = Line::from(entry.spans(styles));
            if entry_line.spans.is_empty() {
                continue;
            }
            if entry_line.width() > width {
                finish_row(&mut rows, &mut row, styles);
                wrap_entry(entry, width, styles, &mut rows);
                continue;
            }
            if !row.spans.is_empty() && row.width() + 2 + entry_line.width() > width {
                finish_row(&mut rows, &mut row, styles);
            }
            if !row.spans.is_empty() {
                row.spans.push(Span::styled("  ", styles.label));
            }
            row.spans.extend(entry_line.spans);
        }
        finish_row(&mut rows, &mut row, styles);
        rows
    }

    pub fn height(&self, width: u16) -> u16 {
        self.rows(width).len().min(usize::from(u16::MAX)) as u16
    }

    /// Render measured rows from the top of the supplied area, leaving the
    /// surrounding surface untouched. Unused rows below are untouched too.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        let styles = self.styles();
        paint_rows(self.layout(area.width, styles), area, buf);
    }
}

impl Widget for &HotkeyBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        HotkeyBar::render(self, area, buf);
    }
}

fn finish_row(rows: &mut Vec<Line<'static>>, row: &mut Line<'static>, styles: Styles) {
    if row.spans.is_empty() {
        return;
    }
    let mut spans = vec![Span::styled(" ", styles.label)];
    spans.append(&mut row.spans);
    spans.push(Span::styled(" ", styles.label));
    rows.push(Line::from(spans).centered());
}

fn wrap_entry(entry: &Hotkey, width: usize, styles: Styles, rows: &mut Vec<Line<'static>>) {
    let mut row = Line::default();
    for token in entry.tokens(styles) {
        let token_width = Line::from(token.clone()).width();
        let separator = usize::from(!row.spans.is_empty());
        if row.width() + separator + token_width <= width {
            if separator != 0 {
                row.spans.push(Span::styled(" ", styles.label));
            }
            row.spans.push(token);
            continue;
        }
        finish_row(rows, &mut row, styles);
        if token_width <= width {
            row.spans.push(token);
            continue;
        }
        // ratatui already supplies Unicode grapheme segmentation; measuring
        // through Line keeps this consistent with the terminal renderer without
        // adding a second unicode-width version or splitting combining marks.
        for grapheme in token.styled_graphemes(Style::default()) {
            let mut symbol = grapheme.symbol.to_owned();
            let mut cells = Line::from(symbol.clone()).width();
            if cells > width {
                symbol = "�".into();
                cells = 1;
            }
            if row.width() + cells > width {
                finish_row(rows, &mut row, styles);
            }
            row.spans.push(Span::styled(symbol, token.style));
        }
    }
    finish_row(rows, &mut row, styles);
}

fn paint_rows(rows: Vec<Line<'static>>, area: Rect, buf: &mut Buffer) {
    for (offset, row) in rows.into_iter().take(usize::from(area.height)).enumerate() {
        row.render(Rect::new(area.x, area.y + offset as u16, area.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        layout::Alignment,
        style::{Color, Modifier},
    };

    fn styles() -> Styles {
        Styles {
            key: Style::default().fg(Color::Yellow).bg(Color::Red).add_modifier(Modifier::BOLD),
            label: Style::default().fg(Color::White).bg(Color::Blue),
        }
    }

    fn bar(entries: impl IntoIterator<Item = Hotkey>) -> HotkeyBar {
        let styles = styles();
        HotkeyBar::new(entries).with_styles(styles.key, styles.label)
    }

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|span| span.content.as_ref()).collect()
    }

    fn compact(rows: &[Line<'_>]) -> String {
        rows.iter().map(text).collect::<String>().chars().filter(|ch| !ch.is_whitespace()).collect()
    }

    #[test]
    fn symbols_and_chords_are_centralized_and_typed() {
        for (key, symbol) in [
            (KeyCode::Enter, "↵"),
            (KeyCode::Esc, "␛"),
            (KeyCode::Backspace, "⌫"),
            (KeyCode::Tab, "⇥"),
            (KeyCode::Delete, "⌦"),
            (KeyCode::Insert, "⎀"),
            (KeyCode::Home, "↖"),
            (KeyCode::End, "↘"),
            (KeyCode::PageUp, "⇞"),
            (KeyCode::PageDown, "⇟"),
            (KeyCode::Up, "↑"),
            (KeyCode::Down, "↓"),
            (KeyCode::Left, "←"),
            (KeyCode::Right, "→"),
            (KeyCode::Char(' '), "␠"),
        ] {
            assert_eq!(key_symbol(key), symbol);
            assert_eq!(Line::from(symbol).width(), 1);
        }
        for n in 1..=12 {
            assert_eq!(key_symbol(KeyCode::F(n)), format!("F{n}"));
        }
        let arrows = Hotkey::alternatives([KeyCode::Up, KeyCode::Down], "Move");
        assert_eq!(arrows.key_text(), "↑/↓");
        assert!(arrows.matches(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
        assert!(!arrows.matches(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT)));
        let chord = Hotkey::modified(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT, KeyCode::Char('s'), "Save");
        assert_eq!(chord.key_text(), "Ctrl+Alt+Shift+s");
        assert!(chord.matches(KeyEvent::new(KeyCode::Char('s'), chord.modifiers)));
        assert!(!chord.matches(KeyEvent::new_with_kind(KeyCode::Char('s'), chord.modifiers, KeyEventKind::Release)));
        assert_eq!(Hotkey::new(KeyCode::BackTab, "Previous").key_text(), "Shift+⇥");
        assert!(Hotkey::new(KeyCode::BackTab, "Previous").matches(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)));
        assert_eq!(Hotkey::modified(KeyModifiers::SHIFT, KeyCode::BackTab, "Previous").key_text(), "Shift+⇥");
    }

    #[test]
    fn english_and_german_wrap_intact_and_center_every_row() {
        for labels in [["Help", "Edit", "Back"], ["Hilfe", "Bearbeiten", "Zurück"]] {
            let bar = bar([
                Hotkey::new(KeyCode::F(1), labels[0]),
                Hotkey::new(KeyCode::Enter, labels[1]),
                Hotkey::new(KeyCode::Esc, labels[2]),
            ]);
            assert_eq!(bar.height(80), 1);
            assert_eq!(text(&bar.rows(80)[0]), text(&bar.line()));
            let rows = bar.rows(14);
            assert!(rows.len() >= 2);
            for row in &rows {
                assert_eq!(row.alignment, Some(Alignment::Center));
                assert!(row.width() <= 14);
                assert!(!text(row).trim().is_empty());
            }
            for entry in &bar.entries {
                let entry_text = text(&Line::from(entry.spans(styles())));
                assert!(rows.iter().any(|row| text(row).contains(&entry_text)));
            }
            let many = bar.clone().append(bar.clone()).append(bar.clone()).append(bar.clone());
            assert!(many.height(80) >= 2);
            assert_eq!(compact(&many.rows(80)), compact(&[many.line()]));
        }
    }

    #[test]
    fn oversized_labels_preserve_words_graphemes_and_all_hotkeys() {
        let bar = bar([
            Hotkey::new(KeyCode::Enter, "Überlange Beschreibung Öffnen 界面 e\u{301}"),
            Hotkey::modified(KeyModifiers::CONTROL, KeyCode::Char('s'), "Speichern"),
            Hotkey::new(KeyCode::Esc, "Back"),
        ]);
        for width in 4..=80 {
            let rows = bar.rows(width);
            assert_eq!(compact(&rows), compact(&[bar.line()]), "width {width}");
            assert_eq!(usize::from(bar.height(width)), rows.len());
            assert!(rows.iter().all(|row| row.width() <= usize::from(width) && !row.spans.is_empty()));
        }
        assert!(text(&bar.rows(12)[0]).trim_start().starts_with('↵'));
        let unicode = bar.rows(4);
        assert!(unicode.iter().any(|row| text(row).contains("e\u{301}")));
        let wide = super::HotkeyBar::new([Hotkey::new(KeyCode::Enter, "界界")]).with_styles(styles().key, styles().label);
        assert_eq!(wide.rows(4).iter().map(Line::width).collect::<Vec<_>>(), [3, 4, 4]);
        assert_eq!(compact(&wide.rows(3)), "↵��");
    }

    #[test]
    fn empty_and_tiny_areas_are_safe_without_blank_rows() {
        let empty = bar([]);
        assert_eq!(empty.height(80), 0);
        assert!(empty.rows(9).is_empty());
        let bar = bar([Hotkey::new(KeyCode::F(12), "Hilfe"), Hotkey::new(KeyCode::Esc, "Zurück")]);
        // Two of the cells belong to the block's own padding.
        for width in 0..=2 {
            assert_eq!(bar.height(width), 0);
            assert!(bar.rows(width).is_empty());
        }
        assert_eq!(compact(&bar.rows(3)), compact(&[bar.line()]));
        assert!(bar.rows(3).iter().all(|row| row.width() == 3));
        let mut buf = Buffer::empty(Rect::new(3, 4, 8, 6));
        let original = buf.clone();
        empty.render(buf.area, &mut buf);
        bar.render(Rect::new(3, 4, 0, 1), &mut buf);
        bar.render(Rect::new(3, 4, 1, 0), &mut buf);
        assert_eq!(buf, original);
        bar.render(Rect::new(0, 0, 100, 100), &mut buf);
    }

    #[test]
    fn rendering_is_a_centered_block_that_leaves_the_surface_alone() {
        let bar = bar([Hotkey::new(KeyCode::Enter, "Öffnen"), Hotkey::new(KeyCode::Esc, "Zurück")]);
        let mut buf = Buffer::empty(Rect::new(2, 3, 30, 12));
        for cell in &mut buf.content {
            cell.set_symbol(".").set_style(Style::default().bg(Color::Green).add_modifier(Modifier::ITALIC));
        }
        let before = buf.clone();
        let area = Rect::new(6, 5, 14, 7);
        bar.render(area, &mut buf);
        let rows = bar.rows(area.width);
        for y in buf.area.y..buf.area.bottom() {
            for x in buf.area.x..buf.area.right() {
                let row = rows.get(usize::from(y.saturating_sub(area.y)));
                let inside = row.is_some_and(|row| {
                    let start = area.x + (area.width - row.width() as u16) / 2;
                    y >= area.y && (start..start + row.width() as u16).contains(&x)
                });
                if !inside {
                    assert_eq!(buf[(x, y)], before[(x, y)], "outside the hint block at {x},{y}");
                }
            }
        }
        for (offset, row) in rows.iter().enumerate() {
            let y = area.y + offset as u16;
            let start = area.x + (area.width - row.width() as u16) / 2;
            // The block starts with its own padding cell, then the key.
            assert_eq!(buf[(start, y)].bg, Color::Blue);
            assert_eq!(buf[(start + 1, y)].fg, Color::Yellow);
            assert_eq!(buf[(start + 1, y)].bg, Color::Red);
            assert_eq!(buf[(start + row.width() as u16 - 1, y)].bg, Color::Blue);
        }
        let mut widget_buf = before.clone();
        Widget::render(&bar, area, &mut widget_buf);
        assert_eq!(buf, widget_buf);
    }

    #[test]
    fn unicode_rendering_matches_measurement_without_clipping() {
        let bar = bar([
            Hotkey::new(KeyCode::Enter, "界面 öffnen e\u{301}"),
            Hotkey::alternatives([KeyCode::Up, KeyCode::Down], "Auswählen"),
            Hotkey::new(KeyCode::Esc, "Zurück"),
        ]);
        for width in 3..=20 {
            let rows = bar.rows(width);
            let area = Rect::new(4, 6, width, bar.height(width));
            let mut buf = Buffer::empty(Rect::new(2, 3, width + 6, area.height + 5));
            for cell in &mut buf.content {
                cell.set_symbol(".");
            }
            let before = buf.clone();
            bar.render(area, &mut buf);
            for (offset, row) in rows.iter().enumerate() {
                let y = area.y + offset as u16;
                let rendered: String = (area.x..area.right()).map(|x| buf[(x, y)].symbol()).collect();
                let rendered: String = rendered.chars().filter(|ch| !ch.is_whitespace() && *ch != '.').collect();
                assert_eq!(rendered, compact(std::slice::from_ref(row)), "width {width}, row {offset}");
            }
            for y in buf.area.y..buf.area.bottom() {
                for x in buf.area.x..buf.area.right() {
                    if x < area.x || x >= area.right() || y < area.y || y >= area.bottom() {
                        assert_eq!(buf[(x, y)], before[(x, y)]);
                    }
                }
            }
            // Insufficient height clips rows, never spills into neighboring UI.
            let short = Rect::new(area.x, area.y, width, 1);
            let mut short_buf = before.clone();
            bar.render(short, &mut short_buf);
            for y in short.bottom()..short_buf.area.bottom() {
                for x in short_buf.area.x..short_buf.area.right() {
                    assert_eq!(short_buf[(x, y)], before[(x, y)]);
                }
            }
        }
    }

    #[test]
    fn rows_are_one_padded_block_without_a_full_width_background() {
        let bar = bar([Hotkey::new(KeyCode::Enter, "Open"), Hotkey::new(KeyCode::Esc, "Back")]);
        let row = bar.rows(80).remove(0);
        assert_eq!(text(&row), " ↵ Open  ␛ Back ");
        for span in row.spans.iter().filter(|span| span.content.trim().is_empty()) {
            assert_eq!(span.style, styles().label);
        }
        assert_eq!(row.style, Style::default());
        assert_eq!(bar.height(row.width() as u16), 1);
        assert_eq!(bar.height(row.width() as u16 - 1), 2);
    }
}
