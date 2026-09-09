//! What the editor underlines while a program is being typed.

mod common;

use common::Server;
use serde_json::{Value, json};
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};
use tower_lsp::lsp_types::Url;

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
    server.open(uri, "PROCEDURE FooBar(INTEGER a)\n  PRINTLN a\nENDPROC\nBEGIN\n  FooBar()\nEND\n");

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
fn statement_argument_errors_underline_the_statement_name() {
    let (mut server, _) = Server::ready();
    for (prefix_index, first_line) in [";$LANGVERSION 400", ";RUBBELDIEKATZ"].iter().enumerate() {
        for (case_index, (statement, message, name_length)) in [
            ("Log \"Foobar\"", "Not enough arguments passed (Log:1:2)", 3),
            ("Log", "Not enough arguments passed (Log:0:2)", 3),
            ("Log \"Foobar\", 1, 2", "Too many arguments passed (Log:3:2)", 3),
            ("Print", "Too few arguments (Print:1)", 5),
        ]
        .iter()
        .enumerate()
        {
            let uri = format!("file:///tmp/statement-argument-{prefix_index}-{case_index}.pps");
            server.open(&uri, &format!("{first_line}\n\nBEGIN\n  {statement}\nEND\n"));
            let diagnostics = server.diagnostics(&uri);
            let errors = of_severity(&diagnostics, 1);
            assert_eq!(errors.len(), 1, "{diagnostics}");
            assert_eq!(errors[0]["message"], *message, "{diagnostics}");
            assert_eq!(
                errors[0]["range"],
                json!({"start": {"line": 3, "character": 2}, "end": {"line": 3, "character": 2 + name_length}}),
                "{diagnostics}"
            );
        }
    }
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

/// Unlike diagnostics(), this cannot mistake a missing publication for success.
fn published(server: &mut Server, uri: &str, version: i64) -> Value {
    let report = server.notification("textDocument/publishDiagnostics");
    assert_eq!(report["params"]["uri"], uri, "{report}");
    assert_eq!(report["params"]["version"], version, "{report}");
    let diagnostics = report["params"]["diagnostics"].clone();
    assert!(diagnostics.is_array(), "{report}");
    diagnostics
}

/// Expected LSP positions are calculated from the fixture, independently of the
/// engine's character offsets and the server's rope conversion.
fn position(source: &str, byte: usize) -> Value {
    let before = &source[..byte];
    json!({
        "line": before.bytes().filter(|byte| *byte == b'\n').count(),
        "character": before.rsplit('\n').next().unwrap().encode_utf16().count(),
    })
}

fn source_range(source: &str, byte: usize, token: &str) -> Value {
    assert!(source[byte..].starts_with(token));
    json!({"start": position(source, byte), "end": position(source, byte + token.len())})
}

#[test]
fn s1_type_not_comparable_publishes_localized_text_and_stable_code() {
    for (locale, expected) in [
        (
            "en_US.UTF-8",
            "Type Envelope does not support equality because it is or contains a non-comparable host object",
        ),
        (
            "de_DE.UTF-8",
            "Typ Envelope unterstützt keinen Gleichheitsvergleich, da er ein nicht vergleichbares Hostobjekt ist oder enthält",
        ),
    ] {
        let mut server = Server::ready_in_locale(locale);
        let uri = "file:///tmp/s1-not-comparable.pps";
        let source = ";$LANGVERSION 400\nTYPE Payload\n USER Owner\nENDTYPE\nTYPE Envelope\n Payload Items[,]\nENDTYPE\nEnvelope left, right\nPRINTLN left = right\nPRINTLN left <> right\n";
        server.open(uri, source);
        let diagnostics = published(&mut server, uri, 1);
        let errors = of_severity(&diagnostics, 1);
        assert_eq!(errors.len(), 2, "{locale}: {diagnostics}");
        for (diagnostic, operator) in errors.iter().zip(["=", "<>"]) {
            assert_eq!(diagnostic["code"], "ppl.type-not-comparable", "{locale}: {diagnostics}");
            assert_eq!(diagnostic["source"], "ppl", "{locale}: {diagnostics}");
            let text = diagnostic["message"].as_str().unwrap().replace(['\u{2068}', '\u{2069}'], "");
            assert_eq!(text, expected, "{locale}");
            assert_eq!(
                diagnostic["range"],
                source_range(source, source.find(operator).unwrap(), operator),
                "{diagnostics}"
            );
        }
    }
}

#[test]
fn s3_var_alias_publishes_localized_warning_with_stable_code() {
    for (locale, expected) in [
        (
            "en_US.UTF-8",
            "VAR arguments 1 and 2 overlap; reverse copy-out writes the earlier parameter last",
        ),
        (
            "de_DE.UTF-8",
            "VAR-Argumente 1 und 2 überlappen; die umgekehrte Rückschreibreihenfolge schreibt den früheren Parameter zuletzt",
        ),
    ] {
        let mut server = Server::ready_in_locale(locale);
        let uri = "file:///tmp/s3-var-alias.pps";
        let source = ";$LANGVERSION 400\nINTEGER number\nChange(number, number)\nPRINT number\nPROCEDURE Change(VAR INTEGER first, VAR INTEGER second)\nfirst = 1\nsecond = 2\nENDPROC\n";
        server.open(uri, source);
        let diagnostics = published(&mut server, uri, 1);
        assert!(of_severity(&diagnostics, 1).is_empty(), "{diagnostics}");
        let warnings: Vec<_> = of_severity(&diagnostics, 2)
            .into_iter()
            .filter(|warning| warning["code"] == "ppl.var-alias")
            .collect();
        assert_eq!(warnings.len(), 1, "{diagnostics}");
        let warning = &warnings[0];
        assert_eq!(warning["message"].as_str().unwrap().replace(['\u{2068}', '\u{2069}'], ""), expected);
        assert_eq!(warning["range"], source_range(source, source.find(", number").unwrap() + 2, "number"));
    }
}

fn assert_errors(diagnostics: &Value, source: &str, expected: &[(&str, &str)]) {
    let errors = of_severity(diagnostics, 1);
    assert_eq!(errors.len(), expected.len(), "{diagnostics}");
    for (token, message) in expected {
        let range = source_range(source, source.find(token).unwrap(), token);
        let matches: Vec<_> = errors.iter().filter(|error| error["message"] == *message && error["range"] == range).collect();
        assert_eq!(matches.len(), 1, "expected {message} at {range}, got {diagnostics}");
        assert_eq!(matches[0]["source"], "ppl", "{diagnostics}");
    }
}

fn assert_unused_routines(diagnostics: &Value, source: &str, names: &[&str]) {
    let warnings = of_severity(diagnostics, 2);
    let routines: Vec<_> = warnings.iter().filter(|warning| warning["code"] == "ppl.unused-routine").collect();
    assert_eq!(routines.len(), names.len(), "{diagnostics}");
    for name in names {
        let message = format!("Unused FUNCTION/PROCEDURE ({name})");
        let warning = routines
            .iter()
            .find(|warning| warning["message"] == message)
            .unwrap_or_else(|| panic!("{diagnostics}"));
        assert_eq!(warning["source"], "ppl", "{diagnostics}");
        assert_eq!(warning["tags"], json!([1]), "{diagnostics}");
        assert_eq!(warning["range"], source_range(source, source.rfind(name).unwrap(), name), "{diagnostics}");
    }
}

#[test]
fn folding_must_not_hide_source_operand_errors_or_move_utf16_spans() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/source-operands.pps";
    // The non-BMP character precedes an error on the same line, and CRLF must
    // not count as an extra line or column in a diagnostic.
    let source = ";$LANGVERSION 400\r\nPRINTLN \"😀\", 0 * missingProduct\r\nPRINTLN FALSE & missingFlag\r\n";
    server.open(uri, source);
    assert_errors(
        &published(&mut server, uri, 1),
        source,
        &[
            ("missingProduct", "Variable not found (missingProduct)"),
            ("missingFlag", "Variable not found (missingFlag)"),
        ],
    );

    server.change(uri, 2, ";$LANGVERSION 400\r\nPRINTLN \"😀\", 0 * 7\r\nPRINTLN FALSE & TRUE\r\n");
    assert_eq!(published(&mut server, uri, 2), json!([]));
}

#[test]
fn folding_must_not_hide_enum_operator_errors() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/source-enum-operator.pps";
    let source = ";$LANGVERSION 400\nENUM Color\n Red = 1\nENDENUM\nPRINTLN 0 * Color.Red\n";
    server.open(uri, source);
    assert_errors(&published(&mut server, uri, 1), source, &[("*", "Operator * is not defined for custom types")]);
}

#[test]
fn dead_structured_bodies_and_control_expressions_are_all_checked() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/source-dead-errors.pps";
    let source = r#";$LANGVERSION 400
IF FALSE THEN
    PRINT missingBranch
ENDIF
WHILE FALSE DO
    PRINT missingLoop
ENDWHILE
IF TRUE THEN
    PRINT 1
ELSEIF (missingElseIf) THEN
    PRINT 2
ENDIF
REPEAT
    BREAK
UNTIL missingUntil
SELECT CASE missingSelector
CASE ELSE
    PRINT 3
ENDSELECT
"#;
    server.open(uri, source);
    assert_errors(
        &published(&mut server, uri, 1),
        source,
        &[
            ("missingBranch", "Variable not found (missingBranch)"),
            ("missingLoop", "Variable not found (missingLoop)"),
            ("missingElseIf", "Variable not found (missingElseIf)"),
            ("missingUntil", "Variable not found (missingUntil)"),
            ("missingSelector", "Variable not found (missingSelector)"),
        ],
    );
}

#[test]
fn structured_liveness_keeps_label_entry_and_source_references() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/source-label-entry.pps";
    let source = r#";$LANGVERSION 400
GOTO entered
IF FALSE THEN
    DeadCall()
    :entered
    LiveCall()
ENDIF
PROCEDURE DeadCall()
ENDPROC
PROCEDURE LiveCall()
ENDPROC
"#;
    server.open(uri, source);
    let diagnostics = published(&mut server, uri, 1);
    assert_errors(&diagnostics, source, &[]);
    assert_unused_routines(&diagnostics, source, &["DeadCall"]);

    // Dead code is still source code: navigation must keep both the call and
    // its declaration even though the routine is not live.
    let references = server.request(
        "textDocument/references",
        json!({
            "textDocument": {"uri": uri},
            "position": position(source, source.find("DeadCall").unwrap()),
            "context": {"includeDeclaration": true},
        }),
    );
    let mut actual = references.as_array().unwrap().clone();
    let mut expected: Vec<_> = source
        .match_indices("DeadCall")
        .map(|(byte, name)| json!({"uri": uri, "range": source_range(source, byte, name)}))
        .collect();
    actual.sort_by_key(Value::to_string);
    expected.sort_by_key(Value::to_string);
    assert_eq!(actual, expected);
}

#[test]
fn structured_control_expressions_have_independent_liveness() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/source-control-liveness.pps";
    let source = r#";$LANGVERSION 400
IF TRUE THEN
    PRINT 1
ELSEIF (HiddenTest()) THEN
    PRINT 2
ENDIF
REPEAT
    BREAK
UNTIL TrailingTest()
SELECT CASE UnusedSelector()
CASE ELSE
    PRINT 3
ENDSELECT
INTEGER counter
GOTO body
FOR counter = Initial() TO Bound() STEP Stride()
    :body
    PRINT counter
NEXT
FUNCTION HiddenTest() BOOLEAN
    RETURN TRUE
ENDFUNC
FUNCTION TrailingTest() BOOLEAN
    RETURN TRUE
ENDFUNC
FUNCTION UnusedSelector() INTEGER
    RETURN 1
ENDFUNC
FUNCTION Initial() INTEGER
    RETURN 1
ENDFUNC
FUNCTION Bound() INTEGER
    RETURN 2
ENDFUNC
FUNCTION Stride() INTEGER
    RETURN 1
ENDFUNC
"#;
    server.open(uri, source);
    let diagnostics = published(&mut server, uri, 1);
    assert_errors(&diagnostics, source, &[]);
    assert_unused_routines(&diagnostics, source, &["HiddenTest", "TrailingTest", "UnusedSelector", "Initial"]);
}

#[test]
fn routine_liveness_stops_at_break_and_continue_and_recomputes_after_edits() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/source-routine-liveness.pps";
    let source = r#";$LANGVERSION 400
Outer()
PROCEDURE Outer()
    IF FALSE THEN
        Hidden()
    ENDIF
    LOOP
        BREAK
        Hidden()
    ENDLOOP
    REPEAT
        CONTINUE
        Hidden()
    UNTIL TRUE
ENDPROC
PROCEDURE Hidden()
ENDPROC
"#;
    server.open(uri, source);
    let diagnostics = published(&mut server, uri, 1);
    assert_errors(&diagnostics, source, &[]);
    assert_unused_routines(&diagnostics, source, &["Hidden"]);

    let live = source.replacen("IF FALSE", "IF TRUE", 1);
    server.change(uri, 2, &live);
    let diagnostics = published(&mut server, uri, 2);
    assert_errors(&diagnostics, &live, &[]);
    assert_unused_routines(&diagnostics, &live, &[]);

    server.change(uri, 3, source);
    let diagnostics = published(&mut server, uri, 3);
    assert_errors(&diagnostics, source, &[]);
    assert_unused_routines(&diagnostics, source, &["Hidden"]);
}

struct DiagnosticsProject(PathBuf);

impl DiagnosticsProject {
    fn new(main: &str, library: &str) -> Self {
        let unique = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let project = Self(std::env::temp_dir().join(format!("ppl-lsp-source-diagnostics-{}-{unique}", std::process::id())));
        fs::create_dir_all(project.0.join("src")).unwrap();
        fs::write(
            project.0.join("ppl.toml"),
            "[package]\nname = \"source-diagnostics\"\nversion = \"0.1.0\"\nruntime = 400\n[compiler]\nlanguage_version = 400\n",
        )
        .unwrap();
        fs::write(project.0.join("src/main.pps"), main).unwrap();
        fs::write(project.0.join("src/library.pps"), library).unwrap();
        project
    }

    fn uri(&self, relative: &str) -> String {
        Url::from_file_path(self.0.join(relative)).unwrap().to_string()
    }
}

impl Drop for DiagnosticsProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn workspace_source_errors_belong_to_their_file_and_use_open_buffers() {
    let main = "IMPORT Library AS L\nPRINT L.Calculate(), 0 * missingMain\n";
    let library = "MODULE Library\nFUNCTION Calculate() INTEGER\n    RETURN 0 * missingLibrary\nENDFUNC\nENDMODULE\n";
    let project = DiagnosticsProject::new(main, library);
    let main_uri = project.uri("src/main.pps");
    let library_uri = project.uri("src/library.pps");
    let (mut server, _) = Server::ready_at(&project.uri(""));

    // Leave the library closed first: its ranges must use disk text, not the
    // changed file's rope. Each workspace pass must publish both files.
    server.open(&main_uri, main);
    let mut reports = HashMap::new();
    for _ in 0..2 {
        let report = server.notification("textDocument/publishDiagnostics");
        let uri = report["params"]["uri"].as_str().unwrap().to_string();
        assert!(reports.insert(uri, report["params"]["diagnostics"].clone()).is_none(), "{report}");
    }
    assert_errors(&reports[&main_uri], main, &[("missingMain", "Variable not found (missingMain)")]);
    assert_errors(&reports[&library_uri], library, &[("missingLibrary", "Variable not found (missingLibrary)")]);

    let repaired_library = library.replace("missingLibrary", "7");
    server.open(&library_uri, &repaired_library);
    for _ in 0..2 {
        server.notification("textDocument/publishDiagnostics");
    }
    let repaired_main = main.replace("missingMain", "2");
    server.change(&main_uri, 2, &repaired_main);
    reports.clear();
    for _ in 0..2 {
        let report = server.notification("textDocument/publishDiagnostics");
        let uri = report["params"]["uri"].as_str().unwrap().to_string();
        assert!(reports.insert(uri, report["params"]["diagnostics"].clone()).is_none(), "{report}");
    }
    assert_eq!(reports[&main_uri], json!([]));
    assert_eq!(
        reports[&library_uri],
        json!([]),
        "analysis reread the broken disk source instead of the open buffer"
    );
}
