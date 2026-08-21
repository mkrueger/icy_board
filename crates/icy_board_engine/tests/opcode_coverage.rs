//! Guards the PCBoard compatibility tables in `executable::compat` against drift.
//!
//! The curated lists there are consumed at runtime (`ppld --check`) where the
//! sources aren't available. This test rebuilds the same information from the VM
//! dispatch tables plus the handler bodies, so implementing (or breaking) an opcode
//! fails here until the tables are updated.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use icy_board_engine::executable::{
    FUNCTION_DEFINITIONS, ImplStatus, LAST_FUNC, LAST_STMT, PARTIAL_FUNCTIONS, PARTIAL_STATEMENTS, STATEMENT_DEFINITIONS, UNIMPLEMENTED_FUNCTIONS,
    UNIMPLEMENTED_STATEMENTS, UNSUPPORTED_FUNCTIONS, UNSUPPORTED_STATEMENTS,
};

/// Both tables are addressed by opcode value, so a definition that moves silently
/// relabels every opcode after it. Aliases live past the opcode range on purpose.
#[test]
fn definition_tables_are_indexed_by_opcode() {
    for (index, definition) in STATEMENT_DEFINITIONS.iter().enumerate().take(LAST_STMT as usize + 1).skip(1) {
        assert_eq!(definition.opcode as usize, index, "statement '{}' sits at index {index}", definition.name);
    }
    for (index, definition) in FUNCTION_DEFINITIONS.iter().enumerate().take(LAST_FUNC.unsigned_abs() as usize + 1) {
        assert_eq!(
            (definition.opcode as i16).unsigned_abs() as usize,
            index,
            "function '{}' sits at index {index}",
            definition.name
        );
    }
}

fn engine_src(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn read(rel: &str) -> String {
    let path = engine_src(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Maps `OpCode::NAME => predefined_procedures::handler(...)` (or the function
/// equivalent) to `NAME -> handler`.
fn parse_dispatch(src: &str, enum_prefix: &str, module: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in src.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(enum_prefix) else {
            continue;
        };
        let Some((variant, rest)) = rest.split_once("=>") else {
            continue;
        };
        let marker = format!("{module}::");
        let Some(idx) = rest.find(&marker) else {
            continue;
        };
        let after = &rest[idx + marker.len()..];
        let handler: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if handler.is_empty() {
            continue;
        }
        map.insert(normalize(variant.trim()), handler);
    }
    map
}

/// Enum variants are CamelCase for some opcodes (`MoveMsg`, `TPACGet`) while the
/// canonical names may carry underscores (`MOVE_MSG`), so compare on a flat form.
fn normalize(s: &str) -> String {
    s.chars().filter(|c| *c != '_').flat_map(char::to_uppercase).collect()
}

/// Collects handler functions whose body invokes the given stub macro.
fn parse_stubbed_handlers(src: &str, stub_macro: &str) -> BTreeSet<String> {
    let mut stubbed = BTreeSet::new();
    let mut current = None;
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub async fn ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            current = Some(name);
        }
        if trimmed.contains(stub_macro)
            && let Some(name) = &current
        {
            stubbed.insert(name.clone());
        }
    }
    stubbed
}

/// Builds `canonical name -> derived status` for one opcode family.
/// Keyed by the opcode enum variant, since a few definitions spell their display
/// name differently from the variant (e.g. `KBDFILUSED` vs `"KBDFilUsued"`).
fn derive_status(
    dispatch: &BTreeMap<String, String>,
    stubbed: &BTreeSet<String>,
    opcodes: &[(String, String)],
    unsupported: &[&str],
) -> BTreeMap<String, ImplStatus> {
    let mut result = BTreeMap::new();
    for (name, variant) in opcodes {
        let Some(handler) = dispatch.get(variant) else {
            continue;
        };
        let status = if handler == "invalid" {
            ImplStatus::Invalid
        } else if stubbed.contains(handler) {
            ImplStatus::Unimplemented
        } else if unsupported.iter().any(|u| u.eq_ignore_ascii_case(name)) {
            ImplStatus::Unsupported
        } else {
            ImplStatus::Implemented
        };
        result.insert(name.to_string(), status);
    }
    result
}

fn compare(kind: &str, derived: &BTreeMap<String, ImplStatus>, declared_unimplemented: &[&str], declared_partial: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();

    let known: BTreeSet<String> = derived.keys().map(|n| normalize(n)).collect();
    let derived_unimpl: BTreeSet<String> = derived
        .iter()
        .filter(|(_, s)| **s == ImplStatus::Unimplemented)
        .map(|(n, _)| normalize(n))
        .collect();
    let declared: BTreeSet<String> = declared_unimplemented.iter().map(|s| normalize(s)).collect();

    for missing in derived_unimpl.difference(&declared) {
        problems.push(format!(
            "{kind} `{missing}` is stubbed in the VM but missing from UNIMPLEMENTED_{}S",
            kind.to_ascii_uppercase()
        ));
    }
    for stale in declared.difference(&derived_unimpl) {
        if known.contains(stale) {
            problems.push(format!(
                "{kind} `{stale}` is listed as unimplemented but the VM has a real handler - remove it from the list"
            ));
        } else {
            problems.push(format!("{kind} `{stale}` is not a known opcode name - typo in the compat list?"));
        }
    }
    // A name can't be both fully stubbed and merely partial.
    for p in declared_partial {
        if derived_unimpl.contains(&normalize(p)) {
            problems.push(format!("{kind} `{p}` is listed as partial but is actually stubbed"));
        }
    }
    problems
}

#[test]
fn compat_tables_match_implementation() {
    let stmt_dispatch = parse_dispatch(&read("vm/statements/mod.rs"), "OpCode::", "predefined_procedures");
    let stmt_stubs = parse_stubbed_handlers(&read("vm/statements/predefined_procedures.rs"), "unimplemented_stmt!");

    let func_dispatch = parse_dispatch(&read("vm/expressions/mod.rs"), "FuncOpCode::", "predefined_functions");
    let func_stubs = parse_stubbed_handlers(&read("vm/expressions/predefined_functions.rs"), "unimplemented_function!");

    assert!(!stmt_dispatch.is_empty(), "failed to parse statement dispatch table");
    assert!(!func_dispatch.is_empty(), "failed to parse function dispatch table");
    assert!(!stmt_stubs.is_empty(), "failed to parse stubbed statement handlers");
    assert!(!func_stubs.is_empty(), "failed to parse stubbed function handlers");

    let stmt_names: Vec<(String, String)> = STATEMENT_DEFINITIONS
        .iter()
        .map(|d| (d.name.to_string(), normalize(&format!("{:?}", d.opcode))))
        .collect();
    let func_names: Vec<(String, String)> = FUNCTION_DEFINITIONS
        .iter()
        .map(|d| (d.name.to_string(), normalize(&format!("{:?}", d.opcode))))
        .collect();

    let stmt_status = derive_status(&stmt_dispatch, &stmt_stubs, &stmt_names, UNSUPPORTED_STATEMENTS);
    let func_status = derive_status(&func_dispatch, &func_stubs, &func_names, UNSUPPORTED_FUNCTIONS);

    let mut problems = compare("statement", &stmt_status, UNIMPLEMENTED_STATEMENTS, PARTIAL_STATEMENTS);
    problems.extend(compare("function", &func_status, UNIMPLEMENTED_FUNCTIONS, PARTIAL_FUNCTIONS));

    print_summary("statements", &stmt_status);
    print_summary("functions", &func_status);

    assert!(
        problems.is_empty(),
        "executable::compat is out of sync with the VM implementation:\n  {}",
        problems.join("\n  ")
    );
}

fn print_summary(kind: &str, status: &BTreeMap<String, ImplStatus>) {
    let count = |s: ImplStatus| status.values().filter(|v| **v == s).count();
    println!(
        "PPL {kind}: {} dispatched | {} implemented, {} unimplemented, {} unsupported, {} invalid",
        status.len(),
        count(ImplStatus::Implemented),
        count(ImplStatus::Unimplemented),
        count(ImplStatus::Unsupported),
        count(ImplStatus::Invalid),
    );
}
