//! Separate test process: choosing the admin palette cannot affect other tests.
use icy_board_tui::{
    chrome::{key_hint, status_line},
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    save_changes_dialog::SaveChangesDialog,
    theme::{DOS_BLUE, DOS_WHITE, DOS_YELLOW, get_tui_theme, set_admin_theme},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    widgets::{Block, Widget},
};

#[test]
fn polished_form_focus_footer_and_popup_render_in_real_admin_palette() {
    set_admin_theme("POLISHED", &Default::default());
    let theme = get_tui_theme();
    assert_eq!(theme.background.bg, Some(DOS_BLUE));
    let mut menu = ConfigMenu {
        obj: (),
        entry: vec![ConfigEntry::Item(ListItem::new(
            "Board name".into(),
            ListValue::Text(24, TextFlags::None, "IcyBoard".into()),
        ))],
    };
    let mut state = ConfigMenuState::default();
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            Block::new().style(theme.background).render(area, frame.buffer_mut());
            menu.render(Rect::new(3, 3, 70, 15), frame, &mut state);
            key_hint("F1 Hilfe · Enter Bearbeiten · Esc Zurück").render(Rect::new(3, 22, 74, 1), frame.buffer_mut());
            status_line(frame.buffer_mut(), Rect::new(0, 24, 80, 1), "Board name", "12:34:56");
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(3, 22)].fg, DOS_YELLOW);
    assert_eq!(buf[(0, 0)].bg, DOS_BLUE);
    assert!(
        buf.content
            .iter()
            .any(|c| c.bg == theme.selected_item.bg.unwrap() || c.bg == theme.edit_value.bg.unwrap())
    );
    terminal
        .draw(|frame| {
            let area = frame.area();
            Block::new().style(theme.background).render(area, frame.buffer_mut());
            SaveChangesDialog::new().render(frame, area);
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    assert!(
        buf.content
            .iter()
            .any(|c| c.bg == theme.selected_item.bg.unwrap() && c.fg == theme.selected_item.fg.unwrap())
    );
    assert_ne!(theme.selected_item.fg, theme.selected_item.bg);
    assert_eq!(theme.value.fg, Some(DOS_WHITE));
}
