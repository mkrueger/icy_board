//! The options ICBSetup offers that nothing in the board reads, taken from
//! compat/OPTIONS_AUDIT.md. `tests/options_audit.rs` keeps the two in step.

use crate::get_text;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Unread {
    /// Editable, written to icboard.toml, and read by nobody.
    NotReadYet,
    /// Only ever filled in by the PCBOARD.DAT importer.
    ImportedOnly,
}

/// Keyed by the name the option carries in `icboard.toml`, not by the Rust field.
pub const UNREAD_OPTIONS: &[(&str, &str, Unread)] = &[
    ("sysop", "config_color_theme", Unread::NotReadYet),
    ("new_user_settings", "new_user_groups", Unread::NotReadYet),
    ("message", "max_msg_lines", Unread::ImportedOnly),
    ("message", "allow_esc_codes", Unread::ImportedOnly),
    ("message", "scan_all_mail_at_login", Unread::NotReadYet),
    ("message", "allow_carbon_copy", Unread::NotReadYet),
    ("message", "validate_to_name", Unread::NotReadYet),
    ("message", "default_quick_personal_scan", Unread::NotReadYet),
    ("file_transfer", "disallow_batch_uploads", Unread::NotReadYet),
    ("file_transfer", "promote_to_batch_transfers", Unread::NotReadYet),
    ("file_transfer", "upload_credit_time", Unread::NotReadYet),
    ("file_transfer", "upload_credit_bytes", Unread::NotReadYet),
    ("file_transfer", "verify_files_uploaded", Unread::NotReadYet),
    ("file_transfer", "upload_descr_lines", Unread::NotReadYet),
    ("file_transfer", "disable_drive_size_check", Unread::NotReadYet),
    ("file_transfer", "stop_uploads_free_space", Unread::NotReadYet),
    ("system_control", "allow_alias_change", Unread::ImportedOnly),
    ("system_control", "disable_full_record_updating", Unread::NotReadYet),
    ("system_control", "is_multi_lingual", Unread::NotReadYet),
    ("system_control", "enforce_daily_time_limit", Unread::NotReadYet),
    ("system_control", "allow_password_failure_comment", Unread::NotReadYet),
    ("switches", "default_graphics_at_login", Unread::NotReadYet),
    ("switches", "capture_grp_chat_session", Unread::NotReadYet),
    ("switches", "allow_handle_in_grpchat", Unread::NotReadYet),
    ("limits", "keyboard_timeout", Unread::NotReadYet),
    ("limits", "max_number_upload_descr_lines", Unread::NotReadYet),
    ("accounting", "use_money", Unread::NotReadYet),
    ("accounting", "concurrent_tracking", Unread::NotReadYet),
    ("accounting", "ignore_empty_sec_level", Unread::NotReadYet),
    ("accounting", "peak_usage_start", Unread::NotReadYet),
    ("accounting", "peak_usage_end", Unread::NotReadYet),
    ("accounting", "peak_days_of_week", Unread::NotReadYet),
    ("accounting", "peak_holiday_list_file", Unread::NotReadYet),
    ("accounting", "info_file", Unread::NotReadYet),
    ("accounting", "logoff_file", Unread::NotReadYet),
    ("subs", "subscription_length", Unread::ImportedOnly),
    ("subs", "default_expired_level", Unread::ImportedOnly),
    ("qwk_settings", "goodbye_screen", Unread::NotReadYet),
    ("qwk_settings", "news_sceen", Unread::NotReadYet),
    ("sysop_sec", "read_all_comments", Unread::NotReadYet),
    ("sysop_sec", "read_all_mail", Unread::NotReadYet),
    ("sysop_sec", "enter_color_codes_in_messages", Unread::NotReadYet),
    ("sysop_sec", "not_update_msg_read", Unread::NotReadYet),
    ("sysop_sec", "enter_generic_messages", Unread::NotReadYet),
    ("sysop_sec", "overwrite_files_on_uploads", Unread::NotReadYet),
    ("sysop_sec", "set_pack_out_date_on_messages", Unread::NotReadYet),
    ("sysop_sec", "see_all_return_receipts", Unread::NotReadYet),
    ("sysop_sec", "sec_1", Unread::NotReadYet),
    ("sysop_sec", "sec_2", Unread::NotReadYet),
    ("sysop_sec", "sec_3", Unread::NotReadYet),
    ("sysop_sec", "sec_5", Unread::NotReadYet),
    ("sysop_sec", "sec_6", Unread::NotReadYet),
    ("sysop_sec", "sec_7", Unread::NotReadYet),
    ("sysop_sec", "sec_8", Unread::NotReadYet),
    ("sysop_sec", "sec_9", Unread::NotReadYet),
    ("sysop_sec", "sec_11", Unread::NotReadYet),
    ("sysop_sec", "sec_12", Unread::NotReadYet),
    ("sysop_sec", "sec_13", Unread::NotReadYet),
    ("sysop_sec", "sec_14", Unread::NotReadYet),
    ("user_sec", "edit_own_messages", Unread::NotReadYet),
];

/// The struct behind a section is not always named like the section.
fn section_of(property: &str) -> &str {
    match property {
        "subscription_info" => "subs",
        "sysop_command_level" => "sysop_sec",
        "user_command_level" => "user_sec",
        other => other,
    }
}

/// The audit knows the sysop commands as `sec_1` to `sec_14`, the config spells out what they do.
fn option_of(field: &str) -> &str {
    let Some(rest) = field.strip_prefix("sec_") else {
        return field;
    };
    let digits = rest.split('_').next().unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return field;
    }
    &field[..4 + digits.len()]
}

pub fn unread(property: &str, field: &str) -> Option<Unread> {
    let (section, option) = (section_of(property), option_of(field));
    UNREAD_OPTIONS.iter().find(|(s, o, _)| *s == section && *o == option).map(|(_, _, kind)| *kind)
}

/// What to tell the sysop about an option the board ignores.
pub fn inactive_reason(property: &str, field: &str) -> Option<String> {
    unread(property, field).map(|kind| match kind {
        Unread::NotReadYet => get_text("option_not_read_yet"),
        Unread::ImportedOnly => get_text("option_imported_only"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sysop_command_is_found_by_its_number() {
        assert_eq!(option_of("sec_14_drop_alt_node_to_dos"), "sec_14");
        assert_eq!(option_of("sec_4_recover_deleted_msg"), "sec_4");
        assert_eq!(option_of("security_file_path"), "security_file_path");
    }

    #[test]
    fn the_renamed_sections_are_found() {
        assert!(unread("subscription_info", "subscription_length").is_some());
        assert!(unread("sysop_command_level", "sec_13_view_alt_node_callers").is_some());
        assert!(unread("user_command_level", "edit_own_messages").is_some());
    }

    #[test]
    fn an_option_the_board_reads_is_not_listed() {
        assert!(unread("board", "name").is_none());
        assert!(unread("sysop_command_level", "sec_4_recover_deleted_msg").is_none());
        assert!(unread("event", "enabled").is_none());
    }
}
