//! Shared DOS chrome: no key remapping, data mutation or additional screen rows.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Widget},
};

use crate::theme::{DOS_DARK_GRAY, get_tui_theme};

pub fn dirty_title(title: impl Into<String>, dirty: bool) -> String {
    let mut title = title.into();
    if dirty {
        title.push_str(" *");
    }
    title
}

fn is_key(token: &str) -> bool {
    let key = token.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | ',' | ':'));
    matches!(
        key,
        "Esc"
            | "ESC"
            | "Enter"
            | "Return"
            | "Tab"
            | "Ins"
            | "Insert"
            | "Del"
            | "Delete"
            | "Entf"
            | "Einfg"
            | "PgUp"
            | "PgDn"
            | "Home"
            | "End"
            | "Pos1"
            | "Ende"
            | "Space"
            | "↑"
            | "↓"
            | "←"
            | "→"
            | "↑↓"
            | "←→"
    ) || key.strip_prefix('F').is_some_and(|n| n.parse::<u8>().is_ok_and(|n| (1..=12).contains(&n)))
        || key.starts_with("Ctrl+")
        || key.starts_with("Alt+")
        || key.starts_with("Shift+")
}

/// Preserve localized text exactly, while distinguishing key names from labels.
pub fn key_hint(text: impl Into<String>) -> Line<'static> {
    let theme = get_tui_theme();
    let text = text.into();
    let mut spans = Vec::new();
    let mut token = String::new();
    for ch in text.chars().chain(std::iter::once('\0')) {
        if ch.is_whitespace() || matches!(ch, '=' | '/' | '·' | '|' | '\0') {
            if !token.is_empty() {
                let style = if is_key(&token) { theme.key_binding } else { theme.key_binding_description };
                spans.push(Span::styled(std::mem::take(&mut token), style));
            }
            if ch != '\0' {
                spans.push(Span::styled(ch.to_string(), theme.key_binding_description));
            }
        } else {
            token.push(ch);
        }
    }
    Line::from(spans)
}

/// Only the content recedes: the surface keeps its colour, highlights go grey.
pub fn dim_background(buf: &mut Buffer, area: Rect) {
    let theme = get_tui_theme();
    let Some(dimmed) = theme.table_inactive.fg else { return };
    let area = area.intersection(*buf.area());
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let cell = &mut buf[(x, y)];
            let highlighted = cell.bg != Color::Reset && Some(cell.bg) != theme.background.bg;
            let mut style = cell
                .style()
                .fg(dimmed)
                .remove_modifier(Modifier::BOLD | Modifier::REVERSED | Modifier::SLOW_BLINK | Modifier::RAPID_BLINK);
            if highlighted {
                style = style.bg(DOS_DARK_GRAY);
            }
            cell.set_style(style);
        }
    }
}

/// Context wins over the clock on narrow terminals. Layout is in display cells.
pub fn status_line(buf: &mut Buffer, area: Rect, context: &str, clock: &str) {
    let theme = get_tui_theme();
    Block::new().style(theme.status_line_text).render(area, buf);
    let clock_width = Line::raw(clock).width().saturating_add(2);
    let show_clock = area.width as usize >= clock_width + 24;
    let reserved = if show_clock { clock_width as u16 } else { 0 };
    Line::styled(format!(" {context}"), theme.status_line_text).render(Rect::new(area.x, area.y, area.width.saturating_sub(reserved), area.height.min(1)), buf);
    if show_clock {
        Line::styled(format!(" {clock} "), theme.status_line).render(Rect::new(area.right() - reserved, area.y, reserved, area.height.min(1)), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_preserve_localized_text_and_style_keys_separately() {
        let text = " F1=Hilfe  Enter Bearbeiten · Esc Zurück  PgUp/PgDn ";
        let line = key_hint(text);
        assert_eq!(line.to_string(), text);
        assert!(line.spans.iter().any(|s| s.content == "F1" && s.style == get_tui_theme().key_binding));
        assert!(
            line.spans
                .iter()
                .any(|s| s.content == "Hilfe" && s.style == get_tui_theme().key_binding_description)
        );
    }

    #[test]
    fn status_is_bounded_and_context_survives_tiny_areas() {
        for width in [0, 1, 8, 25, 80, 120] {
            let area = Rect::new(2, 1, width, 1);
            let mut buf = Buffer::empty(Rect::new(0, 0, 125, 3));
            status_line(&mut buf, area, "Änderungen", "12:34:56");
            assert_eq!(buf[(0, 0)].symbol(), " ");
            if width >= 25 {
                assert_eq!(buf[(3, 1)].symbol(), "Ä");
            }
        }
    }

    #[test]
    fn backdrop_keeps_surface_greys_highlights_and_stays_in_bounds() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));
        buf.set_string(0, 0, "Header", get_tui_theme().app_title);
        buf.set_string(2, 1, "Tab", get_tui_theme().tabs_selected);
        buf[(6, 1)].set_style(get_tui_theme().background);
        let before = buf.clone();
        dim_background(&mut buf, Rect::new(2, 0, 20, 2));
        assert_eq!(buf[(0, 0)], before[(0, 0)]);
        assert_eq!(buf[(2, 1)].symbol(), before[(2, 1)].symbol());
        assert_eq!(buf[(2, 1)].bg, DOS_DARK_GRAY);
        assert_eq!(buf[(6, 1)].bg, before[(6, 1)].bg);
        assert_eq!(buf[(2, 1)].fg, get_tui_theme().table_inactive.fg.unwrap());
        assert!(!buf[(2, 1)].modifier.contains(Modifier::BOLD));
        // Untouched cells keep the terminal's own background.
        assert_eq!(buf[(7, 1)].bg, Color::Reset);
        assert_eq!(dirty_title("Editor", true), "Editor *");
        assert_eq!(dirty_title("Editor", false), "Editor");
    }
}
