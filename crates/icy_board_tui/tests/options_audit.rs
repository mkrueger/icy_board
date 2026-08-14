//! The audit is the list; the table in the code has to match it, or ICBSetup starts
//! greying out the wrong switches.

use icy_board_tui::inactive_options::{UNREAD_OPTIONS, Unread, UnreadOption};

const AUDIT: &str = "../../compat/OPTIONS_AUDIT.md";

fn backticked(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .filter(|name| !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
        .map(str::to_string)
        .collect()
}

fn from_audit() -> Vec<(String, String, Unread, String)> {
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
        // A two column table carries what the audit has to say in the status cell.
        let mut note: String = cells[1].chars().skip(1).collect::<String>().trim().to_string();
        if note.is_empty() {
            note = cells.get(2).unwrap_or(&"").to_string();
        }
        let note = note.replace('`', "");
        let section = if cells[0].contains("in `user_sec`") { "user_sec" } else { section };
        for name in backticked(cells[0]) {
            if name == "user_sec" || name == "sysop_sec" {
                continue;
            }
            found.push((section.to_string(), name, kind, note.clone()));
        }
    }
    found.sort();
    found
}

#[test]
fn the_table_says_what_the_audit_says() {
    let audit = from_audit();
    assert!(audit.len() > 50, "the audit was not read properly, found {}", audit.len());

    let mut table: Vec<(String, String, Unread, String)> = UNREAD_OPTIONS
        .iter()
        .map(|UnreadOption { section, option, kind, note }| (section.to_string(), option.to_string(), *kind, note.to_string()))
        .collect();
    table.sort();

    let missing: Vec<_> = audit.iter().filter(|entry| !table.contains(entry)).collect();
    let extra: Vec<_> = table.iter().filter(|entry| !audit.contains(entry)).collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "compat/OPTIONS_AUDIT.md and inactive_options.rs disagree.\nonly in the audit: {missing:#?}\nonly in the table: {extra:#?}"
    );
}

/// Greying an entry out by hand bypasses the audit, and then nobody notices when the
/// board starts reading the option after all - which is how the e-mail message base
/// ended up greyed out while the mail commands were using it.
#[test]
fn nothing_is_greyed_out_by_hand() {
    let mut marked = Vec::new();
    let mut stack = vec![std::path::PathBuf::from("../icbsetup/src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let source = std::fs::read_to_string(&path).unwrap_or_default();
                for (number, line) in source.lines().enumerate() {
                    if line.contains(".with_inactive(") || line.contains(".with_editable(false)") {
                        marked.push(format!("{}:{}", path.display(), number + 1));
                    }
                }
            }
        }
    }
    assert!(
        marked.is_empty(),
        "an option is greyed out by hand instead of through compat/OPTIONS_AUDIT.md:\n{}",
        marked.join("\n")
    );
}
