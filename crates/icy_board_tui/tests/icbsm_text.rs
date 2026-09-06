use i18n_embed::{
    LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use icy_board_tui::get_text;

#[derive(rust_embed::RustEmbed)]
#[folder = "i18n"]
struct Localizations;

fn loader(locale: &str) -> FluentLanguageLoader {
    let loader = fluent_language_loader!();
    loader.load_languages(&Localizations, &[locale.parse().unwrap()]).unwrap();
    loader.set_use_isolating(false);
    loader
}

#[test]
fn icbsm_confirmation_text_is_available() {
    for (locale, question, keys) in [
        ("en", "Are you sure?", "PGDN=Yes   ESC=Abort"),
        ("de", "Sind Sie sicher?", "PGDN=Ja   ESC=Abbrechen"),
    ] {
        let loader = loader(locale);
        assert_eq!(question, loader.get("icbsm_are_you_sure"));
        assert_eq!(keys, loader.get("icbsm_question_keys"));
    }
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
    for (key, english, german) in [
        (
            "icbsm_table_help_file_ratio",
            "uploads divided by downloads",
            "Uploads geteilt durch die Anzahl der Downloads",
        ),
        (
            "icbsm_table_help_byte_ratio",
            "bytes uploaded divided by bytes downloaded",
            "hochgeladener Bytes geteilt durch die Anzahl heruntergeladener Bytes",
        ),
        ("icbsm_table_help_uploads", "Uploads   Security", "Uploads   Stufe"),
        ("icbsm_table_help_downloads", "Downloads   Security", "Downloads   Stufe"),
    ] {
        for (locale, expected) in [("en", english), ("de", german)] {
            let text = loader(locale).get(key);
            assert!(text.contains(expected), "{locale}/{key} did not parse: {text}");
        }
    }
}
