use icy_board_tui::get_text;

#[test]
fn icbsm_confirmation_text_is_available() {
    assert_eq!("Are you sure?", get_text("icbsm_are_you_sure"));
    assert_eq!("PGDN=Yes   ESC=Abort", get_text("icbsm_question_keys"));
}

#[test]
fn icbsm_main_menu_text_is_available() {
    for key in [
        "icbsm_menu_edit_users",
        "icbsm_menu_sort",
        "icbsm_menu_pack",
        "icbsm_menu_adjust_security",
        "icbsm_menu_insert_conf",
        "icbsm_menu_remove_conf",
        "icbsm_menu_move_conf",
        "icbsm_menu_expiration",
        "icbsm_menu_phones",
        "icbsm_menu_undo",
        "icbsm_menu_groups",
    ] {
        assert!(!get_text(key).is_empty(), "{key} is missing");
    }
}

#[test]
fn icbsm_table_help_text_parses() {
    for (key, expected) in [
        ("icbsm_table_help_file_ratio", "uploads divided by downloads"),
        ("icbsm_table_help_byte_ratio", "bytes uploaded divided by bytes downloaded"),
        ("icbsm_table_help_uploads", "Uploads   Security"),
        ("icbsm_table_help_downloads", "Downloads   Security"),
    ] {
        let text = get_text(key);
        assert!(text.contains(expected), "{key} did not parse: {text}");
    }
}
