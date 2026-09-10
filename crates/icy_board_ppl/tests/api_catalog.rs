//! Freezes the host API surface as reviewable text, so a change to it has to be
//! a decision rather than an accident.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use icy_board_ppl::compiler::user_data::UserDataEntry;
use icy_board_ppl::executable::VariableType;
use icy_board_ppl::parser::UserTypeRegistry;

const SNAPSHOT: &str = include_str!("api_catalog.txt");

const HEADER: &str = "\
# The PPL host API as it is frozen for 4.00.
#
# Member ids may move: a stored program binds by qualified name and signature,
# not by id, so reordering is safe and is covered by imports400's bind tests.
# What this file guards is the surface itself. Removing or renaming a type,
# member or enum variant makes stored programs fail to load, and so does
# changing a parameter that a stored call already passes. Appending an optional
# parameter is allowed and stays loadable.
#
# Update with: UPDATE_API_CATALOG=1 cargo test -p icy_board_ppl --test api_catalog
";

fn type_names(registry: &UserTypeRegistry) -> BTreeMap<u32, String> {
    let mut names = BTreeMap::new();
    for (name, &var_type) in &registry.registered_types {
        if let VariableType::UserData(id) = var_type {
            names.insert(id, name.to_string());
        }
    }
    for definition in registry.enums() {
        names.insert(definition.id, definition.name.to_string());
    }
    names
}

fn render(var_type: VariableType, names: &BTreeMap<u32, String>) -> String {
    match var_type {
        VariableType::UserData(id) => names.get(&id).cloned().unwrap_or_else(|| format!("UserData({id})")),
        other => format!("{other:?}"),
    }
}

fn parameters(names: &BTreeMap<u32, String>, types: &[VariableType], parameter_names: &[String], required: usize) -> String {
    types
        .iter()
        .enumerate()
        .map(|(index, var_type)| {
            let name = parameter_names.get(index).map_or("_", String::as_str);
            let optional = if index >= required { "?" } else { "" };
            format!("{name}{optional}: {}", render(*var_type, names))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn catalog() -> String {
    let registry = UserTypeRegistry::icy_board_registry();
    let names = type_names(&registry);
    let mut out = String::from(HEADER);

    for (&id, object) in registry.types.iter().collect::<BTreeMap<_, _>>() {
        let type_name = names.get(&id).cloned().unwrap_or_else(|| format!("UserData({id})"));
        let _ = write!(out, "\nTYPE {type_name} ({id})");
        if object.instance_provider.is_some() {
            out.push_str(" [instance]");
        }
        if object.static_receiver.is_some() {
            out.push_str(" [static]");
        }
        out.push('\n');

        for (member_id, entry) in object.id_table.iter().enumerate() {
            let name = match entry {
                UserDataEntry::Field(name) | UserDataEntry::Getter(name) | UserDataEntry::Procedure(name) | UserDataEntry::Function(name) => name,
            };
            let is_static = if object.statics.contains(name) { "static " } else { "" };
            let line = match entry {
                UserDataEntry::Field(name) | UserDataEntry::Getter(name) => {
                    let kind = if matches!(entry, UserDataEntry::Field(_)) { "field " } else { "getter" };
                    let var_type = render(object.fields[name], &names);
                    let rank = object.field_ranks.get(name).map_or(String::new(), |rank| format!("[{rank}]"));
                    format!("{kind} {is_static}{name}: {var_type}{rank}")
                }
                UserDataEntry::Procedure(name) => {
                    let member = &object.procedures[name];
                    let args = parameters(&names, &member.parameters, &member.parameter_names, member.required);
                    format!("method {is_static}{name}({args})")
                }
                UserDataEntry::Function(name) => {
                    let member = &object.functions[name];
                    let args = parameters(&names, &member.parameters, &member.parameter_names, member.required);
                    let rank = if member.return_rank > 0 {
                        format!("[{}]", member.return_rank)
                    } else {
                        String::new()
                    };
                    format!("func   {is_static}{name}({args}) -> {}{rank}", render(member.return_type, &names))
                }
            };
            let _ = writeln!(out, "  {member_id:3} {line}");
        }
    }

    let mut enums = registry.enums();
    enums.sort_by_key(|definition| definition.id);
    for definition in enums {
        let _ = write!(out, "\nENUM {} ({})\n  ", definition.name, definition.id);
        let variants = definition
            .variants
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "{variants}");
    }
    out
}

#[test]
fn the_host_api_matches_the_frozen_catalog() {
    let current = catalog();
    if std::env::var("UPDATE_API_CATALOG").is_ok() {
        std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/api_catalog.txt"), &current).unwrap();
        return;
    }
    if current != SNAPSHOT {
        let expected: Vec<_> = SNAPSHOT.lines().collect();
        let actual: Vec<_> = current.lines().collect();
        let mut report = String::from("the host API no longer matches tests/api_catalog.txt\n");
        for (index, line) in actual.iter().enumerate() {
            match expected.get(index) {
                Some(previous) if previous == line => {}
                Some(previous) => {
                    let _ = writeln!(report, "line {}:\n  was: {previous}\n  now: {line}", index + 1);
                }
                None => {
                    let _ = writeln!(report, "line {}: added {line}", index + 1);
                }
            }
        }
        for (index, line) in expected.iter().enumerate().skip(actual.len()) {
            let _ = writeln!(report, "line {}: removed {line}", index + 1);
        }
        report.push_str("\nIf the change is intended, review it and run with UPDATE_API_CATALOG=1.");
        panic!("{report}");
    }
}

/// The rule behind `Valid`, `OK` and `Success`: a handle you look up says whether
/// it found anything, an answer says whether it is good, a search says whether it
/// hit. Objects you simply have carry none of them.
#[test]
fn validity_members_follow_the_documented_rule() {
    let registry = UserTypeRegistry::icy_board_registry();
    let names = type_names(&registry);
    let mut found: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (&id, object) in &registry.types {
        let type_name = names.get(&id).cloned().unwrap_or_default();
        for member in ["Valid", "OK", "Success"] {
            if object.member_id_lookup.contains_key(&unicase::Ascii::new(member.to_string())) {
                found.entry(type_name.clone()).or_default().push(member);
            }
        }
    }

    let expected: BTreeMap<String, Vec<&str>> = [
        ("Area", vec!["Valid"]),
        ("Audio", vec!["Valid"]),
        ("Conference", vec!["Valid"]),
        ("Directory", vec!["Valid"]),
        ("Door", vec!["Valid"]),
        ("Error", vec!["OK"]),
        ("HttpResponse", vec!["Valid", "OK"]),
        ("Msg", vec!["Valid"]),
        ("Regex", vec!["Valid"]),
        ("RegexMatch", vec!["Success"]),
        ("Surface", vec!["Valid"]),
        ("User", vec!["Valid"]),
    ]
    .into_iter()
    .map(|(name, mut members)| {
        members.sort_unstable();
        (name.to_string(), members)
    })
    .collect();

    let found: BTreeMap<String, Vec<&str>> = found
        .into_iter()
        .map(|(name, mut members)| {
            members.sort_unstable();
            (name, members)
        })
        .collect();

    assert_eq!(found, expected, "a type gained or lost a validity member; see docs/new_ppl.md");
}
