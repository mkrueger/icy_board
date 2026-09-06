use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use icy_board_tui::{get_text, tab_page::Page};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

/// Exercise the real page viewport, including every selectable field's editor.
pub(super) fn assert_labels_fit(mut page: impl Page, keys: &[&str]) {
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    for _ in 0..=keys.len() {
        terminal.draw(|frame| page.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (5..23).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect();
        for key in keys {
            let label = get_text(key);
            let row = rows.iter().find(|row| row.contains(&label)).unwrap_or_else(|| panic!("clipped label: {label}"));
            let (_, rest) = row.split_once(&label).unwrap();
            assert!(rest.trim_start().starts_with(':'), "missing separator after {label}: {row}");
        }
        // Neither an editor nor a table column may paint over the right border.
        for y in 5..23 {
            assert_eq!(buffer[(78, y)], buffer[(78, 3)], "right border overwritten at row {y}");
        }
        page.handle_key_press(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
}
