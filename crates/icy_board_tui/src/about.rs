//! Localized about panel shared by the configuration tools.

use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};

use crate::{
    get_text, get_text_args,
    theme::{DOS_LIGHT_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_WHITE, get_tui_theme},
};

const YEAR: &str = "2024";
const AUTHOR: &str = "Mike Krüger";
const URL: &str = "https://github.com/mkrueger/icy_board";

/// Resolve the four about lines using the application's Fluent message key.
pub fn about_text(application_key: &str, version: &str) -> Vec<String> {
    compose_about_text(application_key, version, get_text, get_text_args)
}

fn compose_about_text(
    application_key: &str,
    version: &str,
    text: impl Fn(&str) -> String,
    text_args: impl Fn(&str, HashMap<String, String>) -> String,
) -> Vec<String> {
    vec![
        text_args(
            "about_version",
            HashMap::from([("application".to_string(), text(application_key)), ("version".to_string(), version.to_string())]),
        ),
        text_args(
            "about_author",
            HashMap::from([("year".to_string(), YEAR.to_string()), ("author".to_string(), AUTHOR.to_string())]),
        ),
        text_args("about_website", HashMap::from([("url".to_string(), URL.to_string())])),
        text("about_updates"),
    ]
}

/// Draw a centered, wrapped about panel entirely within the supplied area.
pub fn render_about(frame: &mut Frame, area: Rect, application_key: &str, version: &str) {
    render_text(frame, area, &about_text(application_key, version));
}

fn render_text(frame: &mut Frame, area: Rect, text: &[String]) {
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }

    let lines: Vec<_> = text.iter().map(|line| ice_text(line)).collect();
    // Include both borders and padding BEFORE clamping, and measure terminal
    // columns rather than UTF-8 bytes (e.g. the German application names).
    let horizontal_padding = if area.width >= 8 { 2 } else { 0 };
    let horizontal_space = 2 + 2 * horizontal_padding;
    let width = lines
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or_default()
        .saturating_add(usize::from(horizontal_space))
        .min(usize::from(area.width)) as u16;
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).wrap(Wrap { trim: true });
    let line_count = paragraph.line_count(width.saturating_sub(horizontal_space));
    // Sacrifice vertical padding, not text, when the available height is tight.
    let vertical_padding = u16::from(line_count.saturating_add(4) <= usize::from(area.height));
    let height = line_count.saturating_add(usize::from(2 + 2 * vertical_padding)).min(usize::from(area.height)) as u16;
    let panel = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);
    let block = Block::new()
        .style(get_tui_theme().dialog_box)
        .padding(Padding::new(horizontal_padding, horizontal_padding, vertical_padding, vertical_padding))
        .borders(Borders::ALL)
        .border_type(BorderType::Double);
    let inner = block.inner(panel);
    frame.render_widget(Clear, panel);
    frame.render_widget(block, panel);
    frame.render_widget(paragraph, inner);
}

/// Preserve the original IceText colors, using byte boundaries only for slicing.
fn ice_text(text: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut color = DOS_WHITE;
    let mut start = 0;
    for (offset, character) in text.char_indices() {
        let next_color = if character.is_uppercase() {
            DOS_LIGHT_GRAY
        } else if character.is_ascii_digit() {
            DOS_LIGHT_CYAN
        } else if character.is_ascii_punctuation() {
            DOS_LIGHT_BLUE
        } else {
            DOS_WHITE
        };
        if color != next_color {
            if start < offset {
                spans.push(Span::styled(text[start..offset].to_string(), Style::new().fg(color)));
            }
            start = offset;
            color = next_color;
        }
    }
    if start < text.len() {
        spans.push(Span::styled(text[start..].to_string(), Style::new().fg(color)));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use i18n_embed::{LanguageLoader, fluent::fluent_language_loader};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;

    fn localized_text(locale: &str, application_key: &str) -> Vec<String> {
        // Independent loaders keep these tests deterministic even when other
        // tests use the process's selected locale in parallel.
        let loader = fluent_language_loader!();
        loader.load_languages(&crate::Localizations, &[locale.parse().unwrap()]).unwrap();
        loader.set_use_isolating(false);
        for key in [application_key, "about_version", "about_author", "about_website", "about_updates"] {
            assert!(loader.has(key), "missing {locale}/{key}");
        }
        compose_about_text(application_key, "0.2.0", |key| loader.get(key), |key, args| loader.get_args(key, args))
    }

    fn normalized(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn rendered_text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .filter(|symbol| !matches!(*symbol, "╔" | "╗" | "╚" | "╝" | "═" | "║"))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn english_and_german_about_text_is_fully_visible_at_80_by_25() {
        for (locale, applications) in [
            (
                "en",
                [
                    "IcyBoard Setup Utility",
                    "IcyBoard System Manager",
                    "ICBTEXT File Generator/Editor",
                    "MNU File Editor",
                ],
            ),
            (
                "de",
                [
                    "IcyBoard-Einrichtung",
                    "IcyBoard-Systemverwaltung",
                    "ICBTEXT-Dateigenerator/-Editor",
                    "MNU-Dateieditor",
                ],
            ),
        ] {
            for (key, application) in ["app_icbsetup", "app_icbsm", "app_mkicbtxt", "app_mkicbmnu"].into_iter().zip(applications) {
                let text = localized_text(locale, key);
                assert_eq!(text[0], format!("{application} v0.2.0"));
                assert!(text[1].contains("2024") && text[1].contains("Mike Krüger") && text[1].contains("icy_board"));
                assert_eq!(text[2], format!("{} {URL}", if locale == "en" { "visit" } else { "Besuche" }));
                assert_eq!(
                    text[3],
                    if locale == "en" {
                        "for the latest version & discussions"
                    } else {
                        "für die aktuelle Version und Diskussionen"
                    }
                );
                let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
                terminal.draw(|frame| render_text(frame, Rect::new(0, 2, 80, 21), &text)).unwrap();
                let actual = rendered_text(terminal.backend().buffer());
                assert_eq!(normalized(&actual), normalized(&text.join(" ")), "{locale}/{key}:\n{actual}");
                assert!(actual.contains("v0.2.0") && actual.contains(URL));
            }
        }
    }

    #[test]
    fn wrapped_german_panel_grows_to_show_every_line() {
        let text = localized_text("de", "app_mkicbtxt");
        // A 40-column text area still fits the unbroken URL, but wraps the prose.
        let mut terminal = Terminal::new(TestBackend::new(46, 25)).unwrap();
        terminal.draw(|frame| render_text(frame, frame.area(), &text)).unwrap();
        let actual = rendered_text(terminal.backend().buffer());
        assert_eq!(normalized(&actual), normalized(&text.join(" ")), "{actual}");
        assert!(actual.contains(URL));
        assert!(actual.lines().filter(|line| !line.trim().is_empty()).count() > text.len());
    }

    #[test]
    fn panel_uses_display_width_and_preserves_colors_and_double_border() {
        let mut terminal = Terminal::new(TestBackend::new(30, 15)).unwrap();
        // Nine UTF-8 bytes, but only six terminal columns.
        let text = vec!["Ü9!é界".to_string()];
        assert_eq!(ice_text(&text[0]).width(), 6);
        terminal.draw(|frame| render_text(frame, frame.area(), &text)).unwrap();
        let buffer = terminal.backend().buffer();
        // Six text columns + four padding columns + two borders, centered.
        assert_eq!(buffer[(9, 5)].symbol(), "╔");
        assert_eq!(buffer[(20, 5)].symbol(), "╗");
        assert_eq!(buffer[(9, 9)].symbol(), "╚");
        assert_eq!(buffer[(20, 9)].symbol(), "╝");
        for (x, symbol, color) in [
            (12, "Ü", DOS_LIGHT_GRAY),
            (13, "9", DOS_LIGHT_CYAN),
            (14, "!", DOS_LIGHT_BLUE),
            (15, "é", DOS_WHITE),
            (16, "界", DOS_WHITE),
        ] {
            assert_eq!(buffer[(x, 7)].symbol(), symbol);
            assert_eq!(buffer[(x, 7)].fg, color);
            assert_eq!(buffer[(x, 7)].bg, get_tui_theme().dialog_box.bg.unwrap());
        }
    }

    #[test]
    fn tiny_and_offset_areas_never_paint_outside_their_bounds() {
        let text = vec!["Übersicht 界 v0.2.0".to_string(), format!("{YEAR} {AUTHOR}"), URL.to_string()];
        for width in 0..=24 {
            for height in 0..=12 {
                let mut terminal = Terminal::new(TestBackend::new(30, 18)).unwrap();
                let area = Rect::new(3, 2, width, height);
                terminal
                    .draw(|frame| {
                        for cell in &mut frame.buffer_mut().content {
                            cell.set_symbol("~");
                        }
                        render_text(frame, area, &text);
                    })
                    .unwrap();
                let buffer = terminal.backend().buffer();
                for y in 0..18 {
                    for x in 0..30 {
                        if x < area.x || x >= area.right() || y < area.y || y >= area.bottom() {
                            assert_eq!(buffer[(x, y)].symbol(), "~", "{area:?} painted ({x}, {y})");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn partially_offscreen_areas_and_empty_text_are_safe() {
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        for area in [Rect::new(8, 3, 20, 20), Rect::new(20, 20, 10, 10), Rect::new(0, 0, 0, 0)] {
            terminal.draw(|frame| render_text(frame, area, &["Übersicht 界".to_string()])).unwrap();
            terminal.draw(|frame| render_text(frame, area, &[])).unwrap();
        }
    }
}
