//! Moving around a program: what a word means, where else it is used, and what
//! renaming it would touch. These go through the real server because the
//! handlers live in the binary.

mod common;

use common::Server;
use serde_json::{Value, json};

const URI: &str = "file:///tmp/navigation.pps";

/// Line 0 declares Greet, line 2 declares `name`, lines 3 and 4 use it, and
/// line 6 implements Greet.
const SOURCE: &str = r#"DECLARE PROCEDURE Greet(STRING who)

STRING name
name = "world"
Greet(name)

PROCEDURE Greet(STRING who)
    PRINTLN who
ENDPROC
"#;

fn opened() -> Server {
    let (mut server, _) = Server::ready();
    server.opened(URI, SOURCE);
    server
}

/// The lines a list of locations or ranges points at.
fn lines(value: &Value) -> Vec<u64> {
    let items = value.as_array().unwrap_or_else(|| panic!("expected a list, got {value}"));
    let mut lines: Vec<u64> = items
        .iter()
        .map(|item| item["range"]["start"]["line"].as_u64().unwrap_or_else(|| panic!("no range in {item}")))
        .collect();
    lines.sort_unstable();
    lines
}

#[test]
fn a_call_leads_to_the_routine_it_calls() {
    let mut server = opened();
    let definition = server.at("textDocument/definition", URI, 4, 0);

    // Either the declaration or the implementation is a useful answer.
    let line = match &definition {
        Value::Array(_) => lines(&definition).first().copied(),
        Value::Object(location) => location["range"]["start"]["line"].as_u64(),
        _ => None,
    };
    assert!(matches!(line, Some(0) | Some(6)), "expected the Greet declaration or body, got {definition}");
}

#[test]
fn every_use_of_a_variable_is_found() {
    let mut server = opened();
    let references = server.request(
        "textDocument/references",
        json!({"textDocument": {"uri": URI}, "position": {"line": 2, "character": 7}, "context": {"includeDeclaration": true}}),
    );

    assert_eq!(lines(&references), vec![2, 3, 4], "{references}");
}

#[test]
fn renaming_a_variable_touches_every_use() {
    let mut server = opened();
    let edit = server.request(
        "textDocument/rename",
        json!({"textDocument": {"uri": URI}, "position": {"line": 2, "character": 7}, "newName": "greeting"}),
    );

    let edits = &edit["changes"][URI];
    assert_eq!(lines(edits), vec![2, 3, 4], "{edit}");
    for change in edits.as_array().unwrap() {
        assert_eq!(change["newText"], "greeting", "{edit}");
    }
}

#[test]
fn the_other_uses_of_a_word_are_highlighted() {
    let mut server = opened();
    let highlights = server.at("textDocument/documentHighlight", URI, 3, 0);

    assert_eq!(lines(&highlights), vec![2, 3, 4], "{highlights}");
}

#[test]
fn a_range_can_be_formatted_on_its_own() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/range.pps";
    server.opened(uri, "INTEGER a\n   a   =   1\n");

    let edits = server.request(
        "textDocument/rangeFormatting",
        json!({
            "textDocument": {"uri": uri},
            "range": {"start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 12}},
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    );

    assert!(
        edits.as_array().is_some_and(|edits| !edits.is_empty()),
        "the crooked line was left alone: {edits}"
    );
}
