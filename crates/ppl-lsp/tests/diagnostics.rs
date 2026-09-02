//! What the editor underlines while a program is being typed.

mod common;

use common::Server;
use serde_json::{Value, json};

/// The messages of a list of diagnostics, for readable failures.
fn messages(diagnostics: &Value) -> Vec<String> {
    diagnostics
        .as_array()
        .map(|list| {
            list.iter()
                .map(|diagnostic| diagnostic["message"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn of_severity(diagnostics: &Value, severity: u64) -> Vec<Value> {
    diagnostics
        .as_array()
        .map(|list| list.iter().filter(|d| d["severity"] == severity).cloned().collect())
        .unwrap_or_default()
}

#[test]
fn a_sound_program_is_not_underlined() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/sound.pps";
    server.open(uri, "PRINTLN \"hello\"\n");

    let diagnostics = server.diagnostics(uri);
    assert_eq!(diagnostics.as_array().map(Vec::len), Some(0), "{:?}", messages(&diagnostics));
}

#[test]
fn a_missing_routine_is_reported_as_an_error() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/broken.pps";
    server.open(uri, "DECLARE PROCEDURE Absent()\nAbsent()\n");

    let diagnostics = server.diagnostics(uri);
    let errors = of_severity(&diagnostics, 1);
    assert!(!errors.is_empty(), "no error was reported, only {:?}", messages(&diagnostics));
    assert_eq!(errors[0]["range"]["start"]["line"], 0, "{}", errors[0]);
    assert_eq!(errors[0]["source"], "ppl", "{}", errors[0]);
}

#[test]
fn a_missing_routine_argument_is_reported_without_stopping_analysis() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/missing-argument.pps";
    server.open(
        uri,
        "PROCEDURE FooBar(INTEGER a)\n  PRINTLN a\nENDPROC\nBEGIN\n  FooBar()\nEND\n",
    );

    let errors = messages(&Value::Array(of_severity(&server.diagnostics(uri), 1)));
    assert!(errors.iter().any(|message| message == "Not enough arguments passed (FooBar:0:1)"), "{errors:?}");
}

#[test]
fn an_unused_variable_is_reported_as_a_warning() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/unused.pps";
    server.open(uri, "INTEGER Spare\nPRINTLN \"hello\"\n");

    let diagnostics = server.diagnostics(uri);
    assert!(
        !of_severity(&diagnostics, 2).is_empty(),
        "no warning was reported, only {:?}",
        messages(&diagnostics)
    );
}

#[test]
fn routines_referenced_only_from_dead_code_are_reported_as_unused() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/dead-routines.pps";
    server.open(
        uri,
        r#"PROCEDURE Dead()
    Helper()
ENDPROC

PROCEDURE Helper()
ENDPROC
"#,
    );

    let diagnostics = server.diagnostics(uri);
    let warnings = messages(&Value::Array(of_severity(&diagnostics, 2)));
    assert!(warnings.iter().any(|message| message.contains("Dead")), "{warnings:?}");
    assert!(warnings.iter().any(|message| message.contains("Helper")), "{warnings:?}");
}

#[test]
fn unreachable_statements_are_still_checked_for_errors() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/unreachable-error.pps";
    server.open(
        uri,
        r#"GOTO Done
PRINT missingValue
:Done
"#,
    );

    let errors = messages(&Value::Array(of_severity(&server.diagnostics(uri), 1)));
    assert!(errors.iter().any(|message| message.contains("missingValue")), "{errors:?}");
}

#[test]
fn editing_a_program_takes_its_diagnostics_back() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/edited.pps";
    server.open(uri, "INTEGER Spare\nPRINTLN \"hello\"\n");
    assert!(!server.diagnostics(uri).as_array().unwrap().is_empty(), "the unused variable was not reported");

    server.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "PRINTLN \"hello\"\n"}]
        }
    }));

    let diagnostics = server.diagnostics(uri);
    assert_eq!(diagnostics.as_array().map(Vec::len), Some(0), "{:?}", messages(&diagnostics));
}
