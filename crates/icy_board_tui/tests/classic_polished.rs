//! Separate test process: choosing the admin palette cannot affect other tests.
use icy_board_tui::{
    chrome::status_line,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    hotkeys::HotkeyBar,
    save_changes_dialog::SaveChangesDialog,
    theme::{CLASSIC_THEME, DOS_BLUE, DOS_WHITE, DOS_YELLOW, POLISHED_THEME, get_tui_theme, set_admin_theme},
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
            HotkeyBar::for_id("icb_setup_key_menu_help")
                .with_styles(theme.key_binding, theme.key_binding_description)
                .render(Rect::new(3, 22, 74, 1), frame.buffer_mut());
            status_line(frame.buffer_mut(), Rect::new(0, 24, 80, 1), "Board name", "12:34:56");
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    let rows = HotkeyBar::for_id("icb_setup_key_menu_help")
        .with_styles(theme.key_binding, theme.key_binding_description)
        .rows(74);
    for (index, row) in rows.iter().enumerate().take(1) {
        let y = 22 + index as u16;
        let x = 3 + (74 - row.width() as u16) / 2;
        assert_eq!(buf[(x + 1, y)].symbol(), "↑");
        assert_eq!(buf[(x + 1, y)].fg, DOS_YELLOW);
        let row_area = Rect::new(3, y, 74, 1);
        let mut expected = ratatui::buffer::Buffer::empty(row_area);
        expected.set_style(row_area, theme.background);
        row.render(row_area, &mut expected);
        for x in x..x + row.width() as u16 {
            assert_eq!(buf[(x, y)], expected[(x, y)]);
        }
    }
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

#[test]
fn shared_presets_wrap_center_and_style_with_explicit_themes() {
    for theme in [CLASSIC_THEME, POLISHED_THEME] {
        for id in [
            "icb_setup_key_menu_create_help",
            "icb_setup_key_conf_list_help",
            "message_box_dismiss",
            "icbsm_menu_keys",
        ] {
            for width in [24, 40, 78] {
                let bar = HotkeyBar::for_id(id).with_styles(theme.key_binding, theme.key_binding_description);
                let area = Rect::new(1, 2, width, 22);
                let mut buffer = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 80, 25));
                buffer.set_style(buffer.area, theme.background);
                bar.render(area, &mut buffer);
                let rows = bar.rows(width);
                for (offset, row) in rows.iter().enumerate() {
                    assert!(row.width() <= width as usize);
                    let y = area.y + offset as u16;
                    let mut x = area.x + (width - row.width() as u16) / 2;
                    // Cells beside the centered block keep the surface colour.
                    if x > area.x {
                        assert_eq!(buffer[(area.x, y)].bg, theme.background.bg.unwrap());
                    }
                    for span in &row.spans {
                        let cells = span.width() as u16;
                        if cells > 0 {
                            assert_eq!(buffer[(x, y)].fg, span.style.fg.unwrap());
                            assert_eq!(buffer[(x, y)].bg, span.style.bg.unwrap());
                        }
                        x += cells;
                    }
                }
                assert!(buffer.content.iter().any(|cell| cell.symbol() == "␛" || cell.symbol() == "q"));
            }
        }
    }
}
