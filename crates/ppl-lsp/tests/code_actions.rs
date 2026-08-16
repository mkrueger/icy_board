//! Quick fixes offered for diagnostics with unambiguous source edits.

mod common;

use common::Server;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn actions(server: &mut Server, uri: &str, diagnostics: Value) -> Value {
    server.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 20, "character": 0}},
            "context": {"diagnostics": diagnostics}
        }),
    )
}

#[test]
fn a_function_closed_with_endproc_can_be_fixed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/end-token.pps";
    server.open(
        uri,
        "DECLARE FUNCTION Answer() INTEGER\nFUNCTION Answer() INTEGER\nRETURN 42\nENDPROC\nPRINTLN Answer()\n",
    );
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    assert_eq!(actions[0]["title"], "Replace ENDPROC with ENDFUNC");
    assert_eq!(actions[0]["edit"]["changes"][uri][0]["newText"], "ENDFUNC");
    assert_eq!(actions[0]["diagnostics"][0]["code"], "ppl.function-closed-with-endproc");
}

#[test]
fn a_procedure_closed_with_endfunc_can_be_fixed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/procedure-end-token.pps";
    server.open(uri, "DECLARE PROCEDURE Show()\nPROCEDURE Show()\nPRINTLN \"hello\"\nENDFUNC\nShow()\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    assert_eq!(actions[0]["title"], "Replace ENDFUNC with ENDPROC");
    assert_eq!(actions[0]["edit"]["changes"][uri][0]["newText"], "ENDPROC");
    assert_eq!(actions[0]["diagnostics"][0]["code"], "ppl.procedure-closed-with-endfunc");
}

#[test]
fn obsolete_braces_can_be_replaced() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/braces.pps";
    server.open(uri, ";$LANGVERSION 340\nINTEGER values{2}\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let replacements: Vec<_> = actions
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["edit"]["changes"][uri][0]["newText"].clone())
        .collect();
    assert!(replacements.contains(&json!("(")), "{actions}");
    assert!(replacements.contains(&json!(")")), "{actions}");
}

#[test]
fn an_obsolete_power_operator_can_be_replaced() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/power.pps";
    server.open(uri, "PRINTLN 2 ** 3\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Replace ** with ^"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["diagnostics"][0]["code"], "ppl.obsolete-pow");
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "^");
}

#[test]
fn a_mismatched_next_identifier_can_be_corrected() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/next-identifier.pps";
    server.open(uri, "INTEGER i\nFOR i = 1 TO 3\n  PRINTLN i\nNEXT j\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["diagnostics"][0]["code"] == "ppl.next-identifier-mismatch"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["title"], "Replace with i");
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "i");
    assert_eq!(action["edit"]["changes"][uri][0]["range"]["start"]["line"], 3);
}

#[test]
fn a_routine_used_as_a_value_can_be_called() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/uncalled-routine.pps";
    server.open(
        uri,
        "DECLARE FUNCTION Answer() INTEGER\nPRINTLN Answer\nFUNCTION Answer() INTEGER\nRETURN 42\nENDFUNC\n",
    );
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Call the routine"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["diagnostics"][0]["code"], "ppl.routine-needs-call");
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], "()");
    assert_eq!(edit["range"]["start"], edit["range"]["end"]);
    assert_eq!(edit["range"]["start"]["line"], 1);
}

#[test]
fn a_routine_that_needs_arguments_is_not_called_blindly() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/uncalled-with-parameters.pps";
    server.open(
        uri,
        "DECLARE FUNCTION Echo(INTEGER Value) INTEGER\nPRINTLN Echo\nFUNCTION Echo(INTEGER Value) INTEGER\nRETURN Value\nENDFUNC\n",
    );
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    assert!(
        actions
            .as_array()
            .is_none_or(|actions| actions.iter().all(|action| action["title"] != "Call the routine")),
        "{actions}"
    );
}

#[test]
fn a_var_parameter_of_a_function_can_be_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/var-parameter.pps";
    server.open(uri, "DECLARE FUNCTION Echo(VAR INTEGER Value) INTEGER\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Remove VAR"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["diagnostics"][0]["code"], "ppl.var-not-allowed");
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"]["start"], json!({"line": 0, "character": 22}));
    assert_eq!(edit["range"]["end"], json!({"line": 0, "character": 26}));
}

#[test]
fn a_duplicate_record_literal_field_can_be_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/duplicate-field.pps";
    server.open(uri, "TYPE Point\n INTEGER X\nENDTYPE\nPoint value\nvalue = Point { X = 1, X = 2 }\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Remove duplicate field"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["diagnostics"][0]["code"], "ppl.duplicate-record-field");
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"]["start"], json!({"line": 4, "character": 21}));
    assert_eq!(edit["range"]["end"], json!({"line": 4, "character": 28}));
}

#[test]
fn an_unused_variable_can_be_removed_from_a_list() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/unused-list.pps";
    server.open(uri, "INTEGER Used, Spare, AlsoUsed\nUsed = 1\nAlsoUsed = Used\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Remove unused variable")
        .unwrap();
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"]["start"]["line"], 0);
    assert_eq!(action["diagnostics"][0]["code"], "ppl.unused-variable");
}

#[test]
fn first_and_last_unused_variables_have_comma_aware_edits() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/unused-ends.pps";
    server.open(uri, "INTEGER First, Used, Last\nUsed = 1\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let edits: Vec<_> = actions
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["title"] == "Remove unused variable")
        .map(|action| action["edit"]["changes"][uri][0].clone())
        .collect();
    assert_eq!(edits.len(), 2, "{actions}");
    assert_eq!(edits[0]["range"]["start"]["character"], 8);
    assert!(edits.iter().any(|edit| edit["range"]["start"]["character"] == 19), "{edits:?}");
}

#[test]
fn an_initialized_unused_variable_is_not_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/initialized-unused.pps";
    server.open(uri, "INTEGER Spare = 42\nPRINTLN \"hello\"\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    assert!(
        actions
            .as_array()
            .is_none_or(|actions| actions.iter().all(|action| action["title"] != "Remove unused variable")),
        "{actions}"
    );
}

#[test]
fn a_missing_function_implementation_can_be_created() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/missing-function.pps";
    server.open(uri, "DECLARE FUNCTION Answer(INTEGER Value) INTEGER\nPRINTLN Answer(42)\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Create missing routine implementation")
        .unwrap();
    assert_eq!(
        action["edit"]["changes"][uri][0]["newText"],
        "\nFUNCTION Answer(INTEGER Value) INTEGER\nENDFUNC\n"
    );
    assert_eq!(action["edit"]["changes"][uri][0]["range"]["start"]["line"], 1);
    assert_eq!(action["diagnostics"][0]["code"], "ppl.missing-implementation");
}

#[test]
fn a_missing_procedure_implementation_can_be_created() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/missing-procedure.pps";
    server.open(uri, "DECLARE PROCEDURE Show(STRING Value)\nShow(\"hello\")\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Create missing routine implementation")
        .unwrap();
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "\nPROCEDURE Show(STRING Value)\nENDPROC\n");
}

#[test]
fn an_unused_label_can_be_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/unused-label.pps";
    server.open(uri, ":Spare\nPRINTLN \"hello\"\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Remove unused label")
        .unwrap();
    assert_eq!(action["edit"]["changes"][uri][0]["range"]["end"]["line"], 1);
    assert_eq!(action["diagnostics"][0]["code"], "ppl.unused-label");
    assert_eq!(action["diagnostics"][0]["tags"], json!([1]));
}

#[test]
fn an_unused_routine_declaration_can_be_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/unused-routine.pps";
    server.open(uri, "DECLARE PROCEDURE Spare()\nPROCEDURE Spare()\nENDPROC\nPRINTLN \"hello\"\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Remove unused routine")
        .unwrap();
    assert_eq!(action["edit"]["changes"][uri].as_array().unwrap().len(), 2);
    assert_eq!(action["edit"]["changes"][uri][0]["range"]["end"]["line"], 1);
    assert_eq!(action["edit"]["changes"][uri][1]["range"]["start"]["line"], 1);
    assert_eq!(action["edit"]["changes"][uri][1]["range"]["end"]["line"], 3);
    assert_eq!(action["diagnostics"][0]["code"], "ppl.unused-routine");
}

#[test]
fn a_misspelled_variable_can_be_replaced() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/variable-spelling.pps";
    server.open(uri, "STRING UserName\nUserName = \"Ada\"\nPRINTLN UserNmae\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["diagnostics"][0]["code"] == "ppl.unknown-identifier")
        .unwrap();
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "UserName");
}

#[test]
fn a_misspelled_enum_member_can_be_replaced() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/enum-spelling.pps";
    server.open(uri, ";$LANGVERSION 350\nENUM Color\n Red\n Green\nENDENUM\nPRINTLN Color.Grean\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["diagnostics"][0]["code"] == "ppl.unknown-enum-member"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "Green");
}

#[test]
fn a_misspelled_record_field_can_be_replaced() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/record-spelling.pps";
    server.open(
        uri,
        "TYPE Point\n INTEGER X\n INTEGER Y\nENDTYPE\nPoint value\nvalue = Point { XX = 1, Y = 2 }\nPRINTLN value.X\n",
    );
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["diagnostics"][0]["code"] == "ppl.unknown-record-field"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "X");
}

#[test]
fn a_misspelled_record_member_can_be_replaced() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/record-member-spelling.pps";
    server.open(uri, "TYPE Point\n INTEGER Position\nENDTYPE\nPoint value\nPRINTLN value.Postion\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["diagnostics"][0]["code"] == "ppl.unknown-record-field"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "Position");
}

#[test]
fn excess_procedure_arguments_can_be_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/procedure-arguments.pps";
    server.open(
        uri,
        "DECLARE PROCEDURE Show(STRING Value)\nShow(\"hello\", 2, 3)\nPROCEDURE Show(STRING Value)\nENDPROC\n",
    );
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Remove excess arguments")
        .unwrap();
    assert_eq!(action["diagnostics"][0]["code"], "ppl.too-many-arguments");
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "");
    assert_eq!(action["edit"]["changes"][uri][0]["range"]["start"]["line"], 1);
}

#[test]
fn excess_function_arguments_can_be_removed() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/function-arguments.pps";
    server.open(
        uri,
        "DECLARE FUNCTION Echo(INTEGER Value) INTEGER\nPRINTLN Echo(1, 2)\nFUNCTION Echo(INTEGER Value) INTEGER\nRETURN Value\nENDFUNC\n",
    );
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics);

    assert!(
        actions.as_array().unwrap().iter().any(|action| action["title"] == "Remove excess arguments"),
        "{actions}"
    );
}

#[test]
fn an_end_statement_can_be_upgraded_to_exit() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/end-statement.pps";
    server.open(uri, "PRINTLN \"hello\"\nEND\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Replace END with EXIT"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    assert_eq!(action["diagnostics"][0]["code"], "ppl.end-is-not-a-statement");
    assert_eq!(action["edit"]["changes"][uri][0]["newText"], "EXIT");
    assert_eq!(action["edit"]["changes"][uri][0]["range"]["start"]["line"], 1);
    assert_eq!(action["isPreferred"], json!(true));
}

#[test]
fn an_end_statement_can_keep_the_old_language_instead() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/end-statement-legacy.pps";
    server.open(uri, "PRINTLN \"hello\"\nEND\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Read this file as language version 330"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], ";$LANGVERSION 330\n");
    assert_eq!(edit["range"]["start"], json!({"line": 0, "character": 0}));
    assert_eq!(edit["range"]["end"], json!({"line": 0, "character": 0}));
    assert_eq!(action["isPreferred"], json!(false));
}

#[test]
fn keeping_the_old_language_rewrites_an_existing_directive() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/end-statement-directive.pps";
    server.open(uri, ";$LANGVERSION 400\nPRINTLN \"hello\"\nEND\n");
    let diagnostics = server.diagnostics(uri);
    let actions = actions(&mut server, uri, diagnostics.clone());

    let action = actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Read this file as language version 330"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"));
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], ";$LANGVERSION 330");
    assert_eq!(edit["range"]["start"], json!({"line": 0, "character": 0}));
    assert_eq!(edit["range"]["end"], json!({"line": 0, "character": 17}));
}

fn upgrade(server: &mut Server, uri: &str, source: &str) -> (Value, Value) {
    server.open(uri, source);
    let diagnostics = server.diagnostics(uri);
    let actions = actions(server, uri, diagnostics.clone());
    (diagnostics, actions)
}

fn upgrade_action<'a>(diagnostics: &Value, actions: &'a Value) -> &'a Value {
    actions
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["title"] == "Upgrade file to language version 400"))
        .unwrap_or_else(|| panic!("diagnostics={diagnostics}, actions={actions}"))
}

fn new_texts(action: &Value, uri: &str) -> Vec<Value> {
    action["edit"]["changes"][uri]
        .as_array()
        .unwrap()
        .iter()
        .map(|edit| edit["newText"].clone())
        .collect()
}

#[test]
fn an_old_file_can_be_upgraded_to_the_current_language() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/upgrade.pps";
    let source = ";$LANGVERSION 330\nINTEGER i\nFOR i = 1 TO 3\n  QUIT\nNEXT i\nPRINTLN 2 ** 3\nEND\n";
    let (diagnostics, actions) = upgrade(&mut server, uri, source);

    let action = upgrade_action(&diagnostics, &actions);
    assert_eq!(action["kind"], "source.upgrade.ppl");
    let texts = new_texts(action, uri);
    assert!(texts.contains(&json!("^")), "{texts:?}");
    assert!(texts.contains(&json!("BREAK")), "{texts:?}");
    assert!(texts.contains(&json!("EXIT")), "{texts:?}");
    assert!(texts.contains(&json!(";$LANGVERSION 400")), "{texts:?}");
}

#[test]
fn upgrading_turns_obsolete_braces_into_parentheses() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/upgrade-braces.pps";
    let (diagnostics, actions) = upgrade(&mut server, uri, ";$LANGVERSION 330\nINTEGER values{2}\n");

    let texts = new_texts(upgrade_action(&diagnostics, &actions), uri);
    assert!(texts.contains(&json!("(")), "{texts:?}");
    assert!(texts.contains(&json!(")")), "{texts:?}");
}

#[test]
fn a_file_that_needs_no_upgrade_is_not_offered_one() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/already-modern.pps";
    let (_, actions) = upgrade(&mut server, uri, "PRINTLN \"hello\"\n");

    assert!(
        actions
            .as_array()
            .is_none_or(|actions| actions.iter().all(|action| action["kind"] != "source.upgrade.ppl")),
        "{actions}"
    );
}

#[test]
fn a_client_asking_only_for_quick_fixes_gets_no_source_action() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/upgrade-filtered.pps";
    server.open(uri, ";$LANGVERSION 330\nPRINTLN 2 ** 3\nEND\n");
    let diagnostics = server.diagnostics(uri);
    let actions = server.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 20, "character": 0}},
            "context": {"diagnostics": diagnostics, "only": ["quickfix"]}
        }),
    );

    assert!(
        actions
            .as_array()
            .is_none_or(|actions| actions.iter().all(|action| action["kind"] != "source.upgrade.ppl")),
        "{actions}"
    );
}

fn project(manifest: &str, source: &str) -> (PathBuf, String, String) {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("ppl-lsp-actions-{}-{unique}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("ppl.toml"), manifest).unwrap();
    fs::write(root.join("src/main.pps"), source).unwrap();
    let root_uri = format!("file://{}", root.display());
    let source_uri = format!("file://{}", root.join("src/main.pps").display());
    (root, root_uri, source_uri)
}

#[test]
fn a_required_runtime_can_be_set_in_the_manifest() {
    let manifest = "[package]\nname = \"runtime-action\"\nversion = \"0.1.0\"\nruntime = 400\n";
    let source = "TYPE Point\n INTEGER X\nENDTYPE\nPoint value\nvalue = Point { X = 1 }\n";
    let (root, root_uri, uri) = project(manifest, source);
    let (mut server, _) = Server::ready_at(&root_uri);
    server.open(&uri, source);
    let diagnostics = server.diagnostics(&uri);
    let actions = actions(&mut server, &uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["diagnostics"][0]["code"] == "ppl.runtime-too-old")
        .unwrap();
    let manifest_uri = format!("file://{}", root.join("ppl.toml").display());
    assert_eq!(action["edit"]["changes"][&manifest_uri][0]["newText"], "401");
    assert_eq!(action["title"], "Set project version to 401");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_required_language_version_can_be_added_to_the_manifest() {
    let manifest = "[package]\nname = \"language-action\"\nversion = \"0.1.0\"\nruntime = 100\n";
    let source = "PRINTLN ISNONSTOP()\n";
    let (root, root_uri, uri) = project(manifest, source);
    let (mut server, _) = Server::ready_at(&root_uri);
    server.open(&uri, source);
    let diagnostics = server.diagnostics(&uri);
    let actions = actions(&mut server, &uri, diagnostics);

    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["diagnostics"][0]["code"] == "ppl.language-version-too-old")
        .unwrap();
    let manifest_uri = format!("file://{}", root.join("ppl.toml").display());
    let text = action["edit"]["changes"][&manifest_uri][0]["newText"].as_str().unwrap();
    assert!(text.contains("[compiler]\nlanguage_version = 200"), "{action}");
    assert_eq!(action["title"], "Set project version to 200");
    fs::remove_dir_all(root).unwrap();
}
