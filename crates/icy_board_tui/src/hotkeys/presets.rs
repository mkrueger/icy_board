//! Central, explicit footer catalog. Keys describe existing input handlers;
//! this registry is presentation only, never an input dispatcher or a parser
//! for the retained legacy Fluent footer strings.
//!
//! Migration notes:
//! - `PRESET_IDS` is public through `hotkeys::presets`; no core re-export needed.
//! - Existing footer IDs remain stable, including `icbmoni_on_note_footer`.
//!   Presets exist only where a tool already showed a hint bar; this migration
//!   restyles those bars, it does not add hints to screens that had none.
//! - `icbsm_user_list_actions` omits F4 so the caller can append
//!   `Hotkey::new(KeyCode::F(4), custom_localized_sort_caption)` without a
//!   duplicate shortcut. Keep filter/search/count context outside the keys.
//! - Log callers show `log_view_navigation` and `log_view_mode_keys` in their
//!   own rows, and only `log_view_search_keys` while typing.
//! - Path edit/create/browse and node-monitor Enter hints are conditional: keep
//!   the existing caller-side state tests. F2 user save requires dirty state;
//!   Enter restore requires a backup; event start still opens confirmation.
//! - `icbsm_done_keys` deliberately promises only Esc: the no-backup undo page
//!   does NOT implement the old "any key" prose. No synthetic "any key" KeyCode.

use crossterm::event::{KeyCode, KeyCode::*};

use super::{Hotkey, HotkeyBar};
use crate::get_text;

fn key(code: KeyCode, label: &str) -> Hotkey {
    Hotkey::new(code, get_text(label))
}

fn alternatives(codes: &[KeyCode], label: &str) -> Hotkey {
    Hotkey::alternatives(codes.iter().copied(), get_text(label))
}

// One declaration generates both the public inventory and the exhaustive match,
// so aliases cannot be forgotten in PRESET_IDS. No prefix/fuzzy ID matching.
macro_rules! catalog {
    ($( $($id:literal)|+ => [$($entry:expr),* $(,)?] ),* $(,)?) => {
        /// All supported IDs, including explicit legacy aliases and new scenes.
        /// These identify typed presets, not necessarily Fluent messages.
        pub const PRESET_IDS: &[&str] = &[$($($id),+),*];

        pub(super) fn for_id(id: &str) -> HotkeyBar {
            match id {
                $($($id)|+ => HotkeyBar::new([$($entry),*]),)*
                _ => panic!("Unknown hotkey preset ID: {id}"),
            }
        }
    };
}

catalog! {
    // Shared menus, forms and dialogs (ConfigMenu, SelectMenu, App).
    "icb_setup_key_menu_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"),
        key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],
    "icb_setup_key_menu_edit_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(F(1), "hotkey_help"),
        key(F(2), "hotkey_edit"), key(F(4), "hotkey_browse"), key(Esc, "hotkey_back"),
    ],
    "icb_setup_key_menu_create_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(F(1), "hotkey_help"),
        key(F(3), "hotkey_new"), key(F(4), "hotkey_browse"), key(Esc, "hotkey_back"),
    ],
    "icb_setup_key_menu_browse_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(F(1), "hotkey_help"),
        key(F(4), "hotkey_browse"), key(Esc, "hotkey_back"),
    ],
    "icb_setup_key_conf_list_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(Enter, "hotkey_edit"),
        key(Insert, "hotkey_new"), key(Delete, "hotkey_delete"),
        alternatives(&[PageUp, PageDown], "hotkey_move"), key(Esc, "hotkey_back"),
    ],
    "icbsm_group_edit_keys" => [key(Esc, "hotkey_back")],
    "message_box_dismiss" => [alternatives(&[Enter, Char(' '), Esc], "hotkey_continue")],
    "mkicbtxt_quit_keys" => [
        alternatives(&[Left, Right], "hotkey_select"), key(Enter, "hotkey_confirm"), key(Esc, "hotkey_cancel"),
    ],
    "path_browser_shortcut" => [key(F(4), "hotkey_browse")],

    // icbsetup list editors and import preview.
    "area_editor_key_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(Enter, "hotkey_edit"),
        key(Insert, "hotkey_new"), key(F(2), "hotkey_import"), key(Delete, "hotkey_delete"),
        alternatives(&[PageUp, PageDown], "hotkey_move"), key(Esc, "hotkey_back"),
    ],
    "area_import_load_help" => [key(F(2), "hotkey_load"), key(F(4), "hotkey_browse"), key(Esc, "hotkey_back")],
    "area_import_preview_help" => [
        alternatives(&[Up, Down], "hotkey_move"), key(Char(' '), "hotkey_select"),
        key(Enter, "hotkey_import"), key(Esc, "hotkey_back"),
    ],
    "doors_editor_key_help" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(F(2), "hotkey_new_door"),
        key(Tab, "hotkey_edit_doors"), key(Esc, "hotkey_back"),
    ],
    "doors_editor_key_help_door" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(Enter, "hotkey_edit"),
        alternatives(&[F(2), Insert], "hotkey_new"), key(Delete, "hotkey_delete"),
        key(Tab, "hotkey_edit_bbslink"), key(Esc, "hotkey_back"),
    ],
    "event_editor_keys" => [
        key(Up, "hotkey_up"), key(Down, "hotkey_down"), key(Enter, "hotkey_edit"),
        key(Insert, "hotkey_new"), key(Delete, "hotkey_delete"),
        alternatives(&[PageUp, PageDown], "hotkey_move"),
    ],
    "event_editor_keys_more" => [
        key(F(5), "hotkey_copy"), key(F(6), "hotkey_history"), key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],
    "event_editor_detail_keys" => [key(F(1), "hotkey_help"), key(Esc, "hotkey_close")],
    "event_editor_setup_keys" => [
        key(F(1), "hotkey_help"), key(F(2), "hotkey_edit_events"), key(F(4), "hotkey_browse"), key(Esc, "hotkey_back"),
    ],
    "event_editor_history_keys" => [
        alternatives(&[Up, Down], "hotkey_select"), alternatives(&[PageUp, PageDown, Home, End], "hotkey_scroll"),
        key(F(6), "hotkey_refresh"), key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],
    "zconnect_form_keys" => [
        alternatives(&[Up, Down], "hotkey_field"), alternatives(&[Left, Right], "hotkey_select"),
        key(F(4), "hotkey_browse"), key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],
    "zconnect_link_keys" => [
        alternatives(&[Up, Down], "hotkey_field"), key(Enter, "hotkey_next_field_or_choice"),
        key(F(2), "hotkey_areas"), key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],
    "zconnect_links_keys" => [
        alternatives(&[Up, Down], "hotkey_select"), key(Insert, "hotkey_new"), key(Enter, "hotkey_edit"),
        key(Delete, "hotkey_delete"), key(F(2), "hotkey_areas"), key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],
    "zconnect_areas_keys" => [
        alternatives(&[Up, Down], "hotkey_select"), key(Insert, "hotkey_new"), key(Enter, "hotkey_edit"),
        key(Delete, "hotkey_delete"), key(F(1), "hotkey_help"), key(Esc, "hotkey_back"),
    ],

    // icbsm. Safe specific dismissal instead of the inaccurate "any key".
    "icbsm_menu_keys" => [
        alternatives(&[Up, Down], "hotkey_select"), key(Enter, "hotkey_select"), key(Esc, "hotkey_back"),
    ],
    "icbsm_table_keys" => [
        alternatives(&[Up, Down], "hotkey_field"), alternatives(&[PageDown, F(2)], "hotkey_save"), key(Esc, "hotkey_back"),
    ],
    "icbsm_question_keys" => [
        alternatives(&[PageDown, Enter, F(2)], "hotkey_confirm"), key(Esc, "hotkey_cancel"),
    ],
    "icbsm_sort_keys" => [
        alternatives(&[Char('r'), Char(' ')], "hotkey_reverse_order"),
        alternatives(&[PageDown, Enter, F(2)], "hotkey_run"), key(Esc, "hotkey_cancel"),
    ],
    "icbsm_criteria_keys" => [alternatives(&[PageDown, F(2)], "hotkey_preview"), key(Esc, "hotkey_cancel")],
    "icbsm_preview_keys" => [alternatives(&[Enter, F(2), PageDown], "hotkey_run"), key(Esc, "hotkey_back")],
    "icbsm_done_keys" => [key(Esc, "hotkey_back")],
    "icbsm_undo_keys" => [key(Enter, "hotkey_restore"), key(Esc, "hotkey_cancel")],
    "icbsm_user_list_actions" => [
        key(F(2), "hotkey_save"), key(F(3), "hotkey_search"),
        key(Insert, "hotkey_new"), key(Delete, "hotkey_delete"), key(Enter, "hotkey_edit"), key(Esc, "hotkey_back"),
    ],
    "icbsm_user_list_search" => [key(Enter, "hotkey_keep_filter"), key(Esc, "hotkey_clear_filter")],

    // mkicbtxt modes: Escape keeps a live filter, but cancels an edit/jump.
    "mkicbtxt_command_keys" => [
        key(F(2), "hotkey_filter"), key(F(3), "hotkey_jump"), key(F(4), "hotkey_restore"),
        key(Enter, "hotkey_edit"), alternatives(&[Char('q'), Esc], "hotkey_exit"),
    ],
    "mkicbtxt_edit_keys" => [
        key(F(2), "hotkey_next_style"), key(F(3), "hotkey_previous_style"), key(F(4), "hotkey_restore"),
        key(Enter, "hotkey_apply"), key(Esc, "hotkey_cancel"),
    ],
    "mkicbtxt_filter_keys" => [alternatives(&[Enter, Esc], "hotkey_back")],
    "mkicbtxt_jump_keys" => [key(Enter, "hotkey_jump"), key(Esc, "hotkey_cancel")],

    // CWS node, statistics, log, status and event monitors.
    "icbmoni_footer" => [
        alternatives(&[Up, Down], "hotkey_select"), alternatives(&[PageUp, PageDown], "hotkey_page"),
        key(Home, "hotkey_first"), key(End, "hotkey_last"), key(Esc, "hotkey_back"),
    ],
    "icbmoni_on_note_footer" => [
        alternatives(&[Up, Down], "hotkey_select"), alternatives(&[PageUp, PageDown], "hotkey_page"),
        key(Home, "hotkey_first"), key(End, "hotkey_last"), key(Enter, "hotkey_monitor"), key(Esc, "hotkey_back"),
    ],
    "icb_system_statistics_footer" => [
        alternatives(&[Up, Down], "hotkey_select"), alternatives(&[PageUp, PageDown], "hotkey_page"),
        key(Home, "hotkey_first"), key(End, "hotkey_last"), key(Delete, "hotkey_reset_stats"), key(Esc, "hotkey_back"),
    ],
    "icb_system_statistics_confirm_reset" => [
        key(Char('y'), "hotkey_reset_stats"), key(Char('n'), "hotkey_cancel"), key(Esc, "hotkey_back"),
    ],
    "system_status_keys" => [
        alternatives(&[Up, Down], "hotkey_scroll"), alternatives(&[PageUp, PageDown], "hotkey_page"),
        key(Home, "hotkey_first"), key(End, "hotkey_last"), key(Esc, "hotkey_back"),
    ],
    "log_view_keys" => [
        key(Tab, "hotkey_source"), key(Char('f'), "hotkey_follow"), key(Char('e'), "hotkey_warnings_errors"),
        key(Char('/'), "hotkey_filter"), key(Esc, "hotkey_back"),
    ],
    "log_view_caller_keys" => [
        key(Tab, "hotkey_source"), key(Char('f'), "hotkey_follow"), key(Char('/'), "hotkey_search"), key(Esc, "hotkey_back"),
    ],
    "log_view_file_keys" => [key(Char('f'), "hotkey_follow"), key(Char('/'), "hotkey_search"), key(Esc, "hotkey_back")],
    "log_view_navigation" => [
        alternatives(&[Up, Down], "hotkey_scroll"), alternatives(&[PageUp, PageDown], "hotkey_page"),
        key(Home, "hotkey_first"), key(End, "hotkey_last"), alternatives(&[Left, Right], "hotkey_pan"),
    ],
    "log_view_search_keys" => [key(Enter, "hotkey_apply"), key(Esc, "hotkey_cancel")],
    "log_view_mode_keys" => [
        key(Char('t'), "hotkey_filter_context"), key(Char('n'), "hotkey_next_match"),
        // The handler distinguishes the character's case, not its modifiers.
        key(Char('N'), "hotkey_previous_match"),
    ],
    "event_runtime_picker_keys" => [
        alternatives(&[Up, Down], "hotkey_select"), key(Enter, "hotkey_request_run"),
        alternatives(&[F(5), Char('r')], "hotkey_refresh_history"), key(Esc, "hotkey_back"),
    ],
    "event_runtime_detail_keys" => [
        key(PageUp, "hotkey_older"), key(PageDown, "hotkey_newer"),
        alternatives(&[Left, Right], "hotkey_scroll_details"), key(Char('l'), "hotkey_output"),
    ],
    "event_runtime_confirm_keys" => [
        alternatives(&[Left, Right, Tab], "hotkey_select"), key(Enter, "hotkey_confirm"), key(Esc, "hotkey_cancel"),
    ],
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use fluent_syntax::{ast, parser};

    use super::*;

    const EN: &str = include_str!("../../i18n/en/icy_board_tui.ftl");
    const DE: &str = include_str!("../../i18n/de/icy_board_tui.ftl");

    fn messages(source: &str) -> BTreeMap<&str, ast::Message<&str>> {
        let resource = parser::parse(source).expect("valid Fluent catalog");
        let mut messages = BTreeMap::new();
        for entry in resource.body {
            if let ast::Entry::Message(message) = entry {
                assert!(messages.insert(message.id.name, message).is_none(), "duplicate Fluent ID");
            }
        }
        messages
    }

    #[test]
    fn catalog_ids_are_unique_and_all_presets_have_real_keys_and_labels() {
        let mut ids = BTreeSet::new();
        for &id in PRESET_IDS {
            assert!(ids.insert(id), "duplicate preset: {id}");
            let bar = for_id(id);
            assert!(!bar.entries.is_empty(), "empty preset: {id}");
            let mut chords = Vec::new();
            for entry in bar.entries {
                assert!(!entry.keys.is_empty() && !entry.label.trim().is_empty(), "{id}");
                for code in entry.keys {
                    assert_ne!(code, KeyCode::Null, "no fake any-key binding: {id}");
                    assert!(!chords.contains(&(code, entry.modifiers)), "duplicate chord: {id}/{code:?}");
                    chords.push((code, entry.modifiers));
                }
            }
        }
    }

    #[test]
    fn fluent_catalog_parity_and_common_action_labels() {
        let en = messages(EN);
        let de = messages(DE);
        assert_eq!(en.keys().collect::<Vec<_>>(), de.keys().collect::<Vec<_>>(), "EN/DE catalog parity");
        let labels: Vec<_> = en.keys().copied().filter(|id| id.starts_with("hotkey_")).collect();
        assert!(labels.len() >= 50);
        for id in labels {
            for catalog in [&en, &de] {
                let value = catalog[id].value.as_ref().expect("action has a label");
                assert!(
                    value
                        .elements
                        .iter()
                        .all(|element| matches!(element, ast::PatternElement::TextElement { value } if !value.trim().is_empty())),
                    "{id}: shared action labels must not require interpolation"
                );
            }
        }
        // Production constructors name labels explicitly; check every reference,
        // including those not in the currently selected process locale.
        for label in include_str!("presets.rs")
            .split('"')
            .filter(|s| s.starts_with("hotkey_") && !s.contains(char::is_whitespace))
        {
            if label != "hotkey_" {
                assert!(en.contains_key(label) && de.contains_key(label), "missing action: {label}");
            }
        }
    }

    #[test]
    fn every_rendered_footer_id_is_registered() {
        for id in [
            "area_import_load_help",
            "area_import_preview_help",
            "log_view_navigation",
            "path_browser_shortcut",
            "message_box_dismiss",
            "icbmoni_footer",
            "icbmoni_on_note_footer",
            "icb_system_statistics_footer",
            "icb_system_statistics_confirm_reset",
            "icbsm_user_list_search",
        ] {
            assert!(PRESET_IDS.contains(&id), "unregistered nonstandard footer: {id}");
        }
    }

    #[test]
    fn corrected_and_conditional_shortcuts_remain_honest() {
        use KeyCode::*;
        let codes = |id| for_id(id).entries.into_iter().flat_map(|entry| entry.keys).collect::<Vec<_>>();
        assert!(codes("area_editor_key_help").contains(&Delete));
        assert!(!codes("area_editor_key_help").contains(&Backspace));
        assert!(!codes("icbmoni_footer").contains(&Enter));
        assert!(codes("icbmoni_on_note_footer").contains(&Enter));
        assert_eq!(codes("icbsm_done_keys"), [Esc]);
        assert!(!codes("icb_system_statistics_confirm_reset").contains(&Enter));
        assert!(codes("icb_system_statistics_confirm_reset").contains(&Char('y')));
        assert!(!codes("icbsm_user_list_actions").contains(&F(4)));
        assert!(codes("mkicbtxt_command_keys").contains(&Char('q')));
        assert!(!codes("mkicbtxt_command_keys").contains(&Char('Q')));
        let composed = for_id("icbsm_user_list_actions").append(HotkeyBar::new([Hotkey::new(F(4), "Sort (Name)")]));
        assert_eq!(composed.entries.iter().filter(|entry| entry.keys.contains(&F(4))).count(), 1);
    }

    #[test]
    #[should_panic(expected = "Unknown hotkey preset ID: undocumented_keys")]
    fn unknown_ids_are_programmer_errors() {
        for_id("undocumented_keys");
    }
}
