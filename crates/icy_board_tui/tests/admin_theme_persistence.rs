//! Isolated process because applying a theme changes shared rendering state.
use icy_board_engine::icy_board::{
    IcyBoardSerializer,
    icb_config::{IcbConfig, PcbScreenColors},
};
use icy_board_tui::theme::{POLISHED_THEME, Theme, get_tui_theme, set_admin_theme};

#[test]
fn saved_theme_matches_preview_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("icyboard.toml");
    let mut custom = PcbScreenColors::default();
    custom.colors[11] = 0x1e;
    for (name, palette) in [
        ("DEFAULT1", PcbScreenColors::default()),
        ("DEFAULT2", PcbScreenColors::default_2()),
        ("BLACK_AND_WHITE", PcbScreenColors::black_and_white()),
        ("CUSTOM", custom),
        ("CUSTOM", PcbScreenColors::default()),
        ("POLISHED", PcbScreenColors::default()),
    ] {
        let mut config = IcbConfig::default();
        config.sysop.config_color_theme = name.into();
        config.sysop.config_color_configuration = palette.clone();
        set_admin_theme(&config.sysop.config_color_theme, &config.sysop.config_color_configuration);
        let preview = get_tui_theme();
        let expected = if name == "POLISHED" { POLISHED_THEME } else { Theme::from_pcboard(&palette) };
        assert_eq!(preview.background, expected.background, "{name}");
        assert_eq!(preview.table, expected.table, "{name}");
        config.save(&path).unwrap();

        // Simulate a new process with an unrelated initial theme.
        set_admin_theme("BLACK_AND_WHITE", &PcbScreenColors::black_and_white());
        let loaded = IcbConfig::load(&path).unwrap();
        assert_eq!(loaded.sysop.config_color_theme, name);
        assert_eq!(loaded.sysop.config_color_configuration.colors, palette.colors);
        set_admin_theme(&loaded.sysop.config_color_theme, &loaded.sysop.config_color_configuration);
        let restarted = get_tui_theme();
        assert_eq!(restarted.background, preview.background, "{name}");
        assert_eq!(restarted.table, preview.table, "{name}");
        assert_eq!(restarted.selected_item, preview.selected_item, "{name}");
        assert_eq!(restarted.edit_value, preview.edit_value, "{name}");
        assert_eq!(restarted.key_binding, preview.key_binding, "{name}");
    }
}
