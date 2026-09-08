//! One DOS surface for the call-wait screen and its runtime monitors.
//! Administration-tool theme selection must not affect this chrome.
use chrono::{
    DateTime, Local,
    format::{Item, StrftimeItems},
};
use icy_board_tui::{
    hotkeys::HotkeyBar,
    theme::{DOS_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_RED, DOS_WHITE, DOS_YELLOW, POLISHED_THEME, Theme},
};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType},
};

pub fn theme() -> Theme {
    Theme {
        dialog_box: Style::new().fg(DOS_YELLOW).bg(DOS_BLUE),
        dialog_box_title: Style::new().fg(DOS_YELLOW).bg(DOS_RED).bold(),
        menu_box: Style::new().fg(DOS_YELLOW).bg(DOS_BLUE),
        menu_box_title: Style::new().fg(DOS_YELLOW).bg(DOS_RED).bold(),
        key_binding: Style::new().fg(DOS_YELLOW).bg(DOS_RED).bold(),
        key_binding_description: Style::new().fg(DOS_YELLOW).bg(DOS_RED),
        config_title: Style::new().fg(DOS_LIGHT_CYAN).bg(DOS_BLUE).bold(),
        table: Style::new().fg(DOS_YELLOW).bg(DOS_BLUE),
        selected_item: Style::new().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY),
        ..POLISHED_THEME
    }
}

pub fn hotkeys(id: &str) -> Line<'static> {
    let theme = theme();
    HotkeyBar::for_id(id).with_styles(theme.key_binding, theme.key_binding_description).line()
}

fn title_text(title: &str) -> &str {
    let title = title.trim();
    // The older monitor translations already include brackets.
    title.strip_prefix('[').and_then(|title| title.strip_suffix(']')).unwrap_or(title).trim()
}

fn title_line(title: &str, width: usize) -> Line<'static> {
    let title = title_text(title);
    let full = format!("[ {title} ]");
    let text = if Line::raw(&full).width() <= width {
        full
    } else if width >= 5 {
        let mut clipped = String::new();
        let mut cells = 0;
        for grapheme in Span::raw(title).styled_graphemes(Style::default()) {
            let next = Line::raw(grapheme.symbol).width();
            if cells + next > width - 5 {
                break;
            }
            clipped.push_str(grapheme.symbol);
            cells += next;
        }
        format!("[ {clipped}… ]")
    } else {
        String::new()
    };
    Line::styled(text, theme().dialog_box_title).centered()
}

/// Inner panels and dialogs share the outer frame's border and title style,
/// but have no duplicate date/clock headers.
pub fn panel(title: &str) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Double)
        .border_style(theme().dialog_box)
        .style(theme().background)
        .title_top(title_line(title, usize::MAX))
}

/// Date left, clock right, title geometrically centered. Explicit alignment on
/// each Line matters: Block::title_alignment changes ALL unaligned titles.
/// On tiny screens omit both timestamps before allowing them to hit the title.
pub fn screen(title: &str, date_format: &str, now: DateTime<Local>, width: u16) -> Block<'static> {
    let date_format = if date_format.is_empty() || StrftimeItems::new(date_format).any(|item| matches!(item, Item::Error)) {
        "%m/%d/%y"
    } else {
        date_format
    };
    let stamp_style = Style::new().fg(DOS_WHITE).bg(DOS_BLUE);
    let date = Line::styled(format!(" {} ", now.format(date_format)), stamp_style).left_aligned();
    let time = Line::styled(now.format(" %H:%M:%S ").to_string(), stamp_style).right_aligned();
    let inner_width = usize::from(width.saturating_sub(2));
    let side_width = date.width().max(time.width()) + 1;
    let title_width = Line::raw(format!("[ {} ]", title_text(title))).width();
    let show_stamps = inner_width >= side_width * 2 + title_width.min(12);
    let available = if show_stamps { inner_width - side_width * 2 } else { inner_width };
    let mut block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(theme().dialog_box)
        .style(theme().background)
        .title_top(title_line(title, available));
    if show_stamps {
        block = block.title_top(date).title_top(time);
    }
    block
}

/// Screen-level assertions also catch content/scrollbars overwriting the frame.
#[cfg(test)]
pub(crate) fn assert_header(buf: &ratatui::buffer::Buffer, area: ratatui::layout::Rect, title: &str) {
    let title = format!("[ {} ]", title_text(title));
    let width = Line::raw(&title).width() as u16;
    let start = area.x + (area.width - width) / 2;
    let actual: String = (start..start + width).map(|x| buf[(x, area.y)].symbol()).collect();
    assert_eq!(actual, title);
    for x in start..start + width {
        assert_eq!(buf[(x, area.y)].bg, DOS_RED);
        assert_eq!(buf[(x, area.y)].fg, DOS_YELLOW);
    }
    let date: String = (area.x + 2..area.x + 6).map(|x| buf[(x, area.y)].symbol()).collect();
    assert_eq!(date, "DATE");
    let time: String = (area.right() - 10..area.right() - 2).map(|x| buf[(x, area.y)].symbol()).collect();
    assert!(chrono::NaiveTime::parse_from_str(&time, "%H:%M:%S").is_ok(), "{time}");
    for (x, y, symbol) in [
        (area.x, area.y, "╔"),
        (area.right() - 1, area.y, "╗"),
        (area.x, area.bottom() - 1, "╚"),
        (area.right() - 1, area.bottom() - 1, "╝"),
    ] {
        assert_eq!(buf[(x, y)].symbol(), symbol);
        assert_eq!(buf[(x, y)].fg, DOS_YELLOW);
        assert_eq!(buf[(x, y)].bg, DOS_BLUE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    #[test]
    fn timestamps_and_title_have_independent_positions_and_colours() {
        let now = Local.with_ymd_and_hms(2026, 9, 8, 9, 10, 11).unwrap();
        for width in [80, 100, 132] {
            let area = Rect::new(3, 2, width, 25);
            let mut buf = Buffer::empty(area);
            screen("[ IcyBoard-Node-Überwachung ]", "%d.%m.%y", now, width)
                .title_bottom(hotkeys("icbmoni_footer"))
                .render(area, &mut buf);
            assert_eq!(buf[(area.x, area.y)].symbol(), "╔");
            assert_eq!(buf[(area.right() - 1, area.y)].symbol(), "╗");
            let date: String = (area.x + 2..area.x + 10).map(|x| buf[(x, area.y)].symbol()).collect();
            let time: String = (area.right() - 10..area.right() - 2).map(|x| buf[(x, area.y)].symbol()).collect();
            assert_eq!(date, "08.09.26");
            assert_eq!(time, "09:10:11");
            let title = "[ IcyBoard-Node-Überwachung ]";
            let left = area.x + (width - Line::raw(title).width() as u16) / 2;
            let rendered: String = (left..left + Line::raw(title).width() as u16).map(|x| buf[(x, area.y)].symbol()).collect();
            assert_eq!(rendered, title);
            assert_eq!(buf[(left, area.y)].fg, DOS_YELLOW);
            assert_eq!(buf[(left, area.y)].bg, DOS_RED);
            assert_eq!(buf[(left - 1, area.y)].symbol(), "═");
            assert_eq!(buf[(left - 1, area.y)].fg, DOS_YELLOW);
            assert_eq!(buf[(left - 1, area.y)].bg, DOS_BLUE);
            assert_eq!(buf[(area.x + 2, area.y)].fg, DOS_WHITE);
            assert_eq!(buf[(area.x, area.bottom() - 1)].symbol(), "╚");
            assert_eq!(buf[(area.x + 1, area.bottom() - 1)].bg, DOS_BLUE);
        }
    }

    #[test]
    fn narrow_headers_never_overwrite_corners_or_escape_the_area() {
        let now = Local.with_ymd_and_hms(2026, 9, 8, 9, 10, 11).unwrap();
        for width in 0..100 {
            for format in ["%d.%m.%Y", "%A, %d. %B %Y", "%", ""] {
                let area = Rect::new(3, 2, width, 5);
                let mut buf = Buffer::empty(Rect::new(0, 0, 110, 10));
                screen("Überwachung 界面 e\u{301} — langer Titel", format, now, width).render(area, &mut buf);
                if width >= 2 {
                    assert_eq!(buf[(area.x, area.y)].symbol(), "╔");
                    assert_eq!(buf[(area.right() - 1, area.y)].symbol(), "╗");
                }
                assert_eq!(buf[(2, 2)].symbol(), " ");
                assert_eq!(buf[(area.right(), 2)].symbol(), " ");
            }
        }
    }
}
