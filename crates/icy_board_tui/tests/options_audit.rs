//! The audit is the list; the table in the code has to match it, or ICBSetup starts
//! greying out the wrong switches.

use icy_board_tui::inactive_options::{UNREAD_OPTIONS, Unread};

const AUDIT: &str = "../../compat/OPTIONS_AUDIT.md";

fn backticked(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .filter(|name| !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
        .map(str::to_string)
        .collect()
}

fn from_audit() -> Vec<(String, String, Unread)> {
    let text = std::fs::read_to_string(AUDIT).unwrap_or_else(|e| panic!("{AUDIT}: {e}"));
    let mut section: Option<&str> = None;
    let mut found = Vec::new();

    for line in text.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            let name = title.split_whitespace().next().unwrap_or_default();
            // The tail of the file talks about PCBoard options that have no home here.
            section = if name.chars().next().is_some_and(char::is_uppercase) {
                None
            } else {
                Some(name)
            };
            continue;
        }
        let (Some(section), true) = (section, line.starts_with('|')) else {
            continue;
        };
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 {
            continue;
        }
        let kind = if cells[1].starts_with('❌') {
            Unread::NotReadYet
        } else if cells[1].starts_with('📥') {
            Unread::ImportedOnly
        } else {
            continue;
        };
        let section = if cells[0].contains("in `user_sec`") { "user_sec" } else { section };
        for name in backticked(cells[0]) {
            if name == "user_sec" || name == "sysop_sec" {
                continue;
            }
            found.push((section.to_string(), name, kind));
        }
    }
    found.sort();
    found
}

#[test]
fn the_table_says_what_the_audit_says() {
    let audit = from_audit();
    assert!(audit.len() > 50, "the audit was not read properly, found {}", audit.len());

    let mut table: Vec<(String, String, Unread)> = UNREAD_OPTIONS
        .iter()
        .map(|(section, option, kind)| (section.to_string(), option.to_string(), *kind))
        .collect();
    table.sort();

    let missing: Vec<_> = audit.iter().filter(|entry| !table.contains(entry)).collect();
    let extra: Vec<_> = table.iter().filter(|entry| !audit.contains(entry)).collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "compat/OPTIONS_AUDIT.md and inactive_options.rs disagree.\nonly in the audit: {missing:#?}\nonly in the table: {extra:#?}"
    );
}
