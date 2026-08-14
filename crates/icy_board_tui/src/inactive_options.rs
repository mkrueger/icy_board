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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct UnreadOption {
    /// The name the option carries in `icboard.toml`, not the Rust field.
    pub section: &'static str,
    pub option: &'static str,
    pub kind: Unread,
    /// What the audit says happens instead, empty when it says nothing.
    pub note: &'static str,
}

impl UnreadOption {
    /// One line for the status bar, in the language of the sysop.
    pub fn reason(&self) -> String {
        match self.kind {
            Unread::NotReadYet => get_text("option_not_read_yet"),
            Unread::ImportedOnly => get_text("option_imported_only"),
        }
    }
}

pub const UNREAD_OPTIONS: &[UnreadOption] = &[
    UnreadOption {
        section: "sysop",
        option: "config_color_theme",
        kind: Unread::NotReadYet,
        note: "the TUI theme is not chosen from it",
    },
    UnreadOption {
        section: "new_user_settings",
        option: "new_user_groups",
        kind: Unread::NotReadYet,
        note: "a new user is never put into the group named here",
    },
    UnreadOption {
        section: "message",
        option: "max_msg_lines",
        kind: Unread::ImportedOnly,
        note: "the editor has its own limit",
    },
    UnreadOption {
        section: "message",
        option: "allow_esc_codes",
        kind: Unread::ImportedOnly,
        note: "ESC is filtered or not without asking this",
    },
    UnreadOption {
        section: "message",
        option: "scan_all_mail_at_login",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "message",
        option: "allow_carbon_copy",
        kind: Unread::NotReadYet,
        note: "E never offers a carbon copy",
    },
    UnreadOption {
        section: "message",
        option: "validate_to_name",
        kind: Unread::NotReadYet,
        note: "a message to a name nobody carries is accepted",
    },
    UnreadOption {
        section: "message",
        option: "default_quick_personal_scan",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "file_transfer",
        option: "disallow_batch_uploads",
        kind: Unread::NotReadYet,
        note: "BU is a stub anyway",
    },
    UnreadOption {
        section: "file_transfer",
        option: "upload_credit_time",
        kind: Unread::NotReadYet,
        note: "uploading earns neither time nor bytes",
    },
    UnreadOption {
        section: "file_transfer",
        option: "upload_credit_bytes",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "file_transfer",
        option: "verify_files_uploaded",
        kind: Unread::NotReadYet,
        note: "uploads are never test-extracted",
    },
    UnreadOption {
        section: "file_transfer",
        option: "disable_drive_size_check",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "file_transfer",
        option: "stop_uploads_free_space",
        kind: Unread::NotReadYet,
        note: "the board uploads until the disk is full",
    },
    UnreadOption {
        section: "system_control",
        option: "allow_alias_change",
        kind: Unread::ImportedOnly,
        note: "W lets the alias be changed regardless",
    },
    UnreadOption {
        section: "system_control",
        option: "disable_full_record_updating",
        kind: Unread::NotReadYet,
        note: "W always asks everything",
    },
    UnreadOption {
        section: "system_control",
        option: "is_multi_lingual",
        kind: Unread::NotReadYet,
        note: "LANG works whether or not this is set",
    },
    UnreadOption {
        section: "system_control",
        option: "enforce_daily_time_limit",
        kind: Unread::NotReadYet,
        note: "only session limits exist",
    },
    UnreadOption {
        section: "system_control",
        option: "allow_password_failure_comment",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "switches",
        option: "default_graphics_at_login",
        kind: Unread::NotReadYet,
        note: "graphics mode is decided by the terminal handshake",
    },
    UnreadOption {
        section: "switches",
        option: "capture_grp_chat_session",
        kind: Unread::NotReadYet,
        note: "group chat is never logged",
    },
    UnreadOption {
        section: "switches",
        option: "allow_handle_in_grpchat",
        kind: Unread::NotReadYet,
        note: "group chat always uses the handle",
    },
    UnreadOption {
        section: "limits",
        option: "keyboard_timeout",
        kind: Unread::NotReadYet,
        note: "an idle user is never disconnected",
    },
    UnreadOption {
        section: "limits",
        option: "max_number_upload_descr_lines",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "accounting",
        option: "use_money",
        kind: Unread::NotReadYet,
        note: "amounts are always shown as units",
    },
    UnreadOption {
        section: "accounting",
        option: "concurrent_tracking",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "accounting",
        option: "ignore_empty_sec_level",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "accounting",
        option: "peak_usage_start",
        kind: Unread::NotReadYet,
        note: "peak rates are never applied",
    },
    UnreadOption {
        section: "accounting",
        option: "peak_usage_end",
        kind: Unread::NotReadYet,
        note: "peak rates are never applied",
    },
    UnreadOption {
        section: "accounting",
        option: "peak_days_of_week",
        kind: Unread::NotReadYet,
        note: "peak rates are never applied",
    },
    UnreadOption {
        section: "accounting",
        option: "peak_holiday_list_file",
        kind: Unread::NotReadYet,
        note: "peak rates are never applied",
    },
    UnreadOption {
        section: "accounting",
        option: "info_file",
        kind: Unread::NotReadYet,
        note: "only the warning file is displayed",
    },
    UnreadOption {
        section: "accounting",
        option: "logoff_file",
        kind: Unread::NotReadYet,
        note: "only the warning file is displayed",
    },
    UnreadOption {
        section: "subs",
        option: "subscription_length",
        kind: Unread::ImportedOnly,
        note: "a new subscription period is never set",
    },
    UnreadOption {
        section: "subs",
        option: "default_expired_level",
        kind: Unread::ImportedOnly,
        note: "an expired user keeps their level",
    },
    UnreadOption {
        section: "qwk_settings",
        option: "goodbye_screen",
        kind: Unread::NotReadYet,
        note: "not packed into the QWK archive",
    },
    UnreadOption {
        section: "qwk_settings",
        option: "news_sceen",
        kind: Unread::NotReadYet,
        note: "not packed into the QWK archive",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "read_all_comments",
        kind: Unread::NotReadYet,
        note: "always granted to whoever passes the sysop level",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "read_all_mail",
        kind: Unread::NotReadYet,
        note: "always granted to whoever passes the sysop level",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "enter_color_codes_in_messages",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "not_update_msg_read",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "enter_generic_messages",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "overwrite_files_on_uploads",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "set_pack_out_date_on_messages",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "see_all_return_receipts",
        kind: Unread::NotReadYet,
        note: "",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_1",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_2",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_3",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_5",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_6",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_7",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_8",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_9",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_11",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_12",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_13",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "sysop_sec",
        option: "sec_14",
        kind: Unread::NotReadYet,
        note: "the numeric command itself is missing, see COMMAND_AUDIT.md",
    },
    UnreadOption {
        section: "user_sec",
        option: "edit_own_messages",
        kind: Unread::NotReadYet,
        note: "",
    },
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

pub fn lookup(property: &str, field: &str) -> Option<&'static UnreadOption> {
    let (section, option) = (section_of(property), option_of(field));
    UNREAD_OPTIONS.iter().find(|o| o.section == section && o.option == option)
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
        assert!(lookup("subscription_info", "subscription_length").is_some());
        assert!(lookup("sysop_command_level", "sec_13_view_alt_node_callers").is_some());
        assert!(lookup("user_command_level", "edit_own_messages").is_some());
    }

    #[test]
    fn an_option_the_board_reads_is_not_listed() {
        assert!(lookup("board", "name").is_none());
        assert!(lookup("sysop_command_level", "sec_4_recover_deleted_msg").is_none());
        assert!(lookup("event", "enabled").is_none());
    }

    #[test]
    fn the_note_of_the_audit_is_kept() {
        assert_eq!(lookup("limits", "keyboard_timeout").unwrap().note, "an idle user is never disconnected");
    }
}
