use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::{CompilerData, Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::{completion::get_completion, signature_help::get_signature_help};
use tower_lsp::lsp_types::ParameterLabel;

/// Parses a source the way the server does and hands back its semantic model.
fn analyze(source: &str) -> (icy_board_engine::ast::Ast, SemanticVisitor) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor)
}

/// Completes at the end of `source`, which is where the cursor is.
fn complete(source: &str) -> Vec<String> {
    let (ast, visitor) = analyze(source);
    let line = source.lines().last().unwrap_or("");
    get_completion(&ast, &visitor, line, source.chars().count())
        .into_iter()
        .map(|item| item.label)
        .collect()
}

fn help(source: &str) -> Vec<String> {
    let (_, visitor) = analyze(source);
    let line = source.lines().last().unwrap_or("");
    get_signature_help(line, &visitor)
        .map(|help| help.signatures.into_iter().map(|s| s.label).collect())
        .unwrap_or_default()
}

const RECORD: &str = r#"TYPE Address
    STRING Town
ENDTYPE
TYPE Member
    STRING Name
    INTEGER Age
    Address Home
ENDTYPE
Member m
Member people(10)
"#;

#[test]
fn a_record_offers_its_fields() {
    let items = complete(&format!("{RECORD}m."));
    assert_eq!(items, vec!["Name".to_string(), "Age".to_string(), "Home".to_string()]);
}

#[test]
fn a_field_of_a_record_offers_its_own_fields() {
    let items = complete(&format!("{RECORD}m.Home."));
    assert_eq!(items, vec!["Town".to_string()]);
}

#[test]
fn an_indexed_record_offers_its_fields() {
    let items = complete(&format!("{RECORD}people[0].Home."));
    assert_eq!(items, vec!["Town".to_string()]);
}

#[test]
fn a_prefix_does_not_hide_the_fields() {
    let items = complete(&format!("{RECORD}PRINTLN m.Na"));
    assert!(items.contains(&"Name".to_string()), "{items:?}");
}

#[test]
fn a_board_object_offers_fields_and_methods() {
    let items = complete("CONFERENCE conf = ConfInfo(CurConf())\nconf.");
    assert!(items.contains(&"Name".to_string()), "{items:?}");
    assert!(items.contains(&"HasAccess".to_string()), "{items:?}");
    assert!(items.contains(&"GetDoor".to_string()), "{items:?}");
}

#[test]
fn a_function_answering_an_object_offers_its_members() {
    let items = complete("ConfInfo(CurConf()).");
    assert!(items.contains(&"Areas".to_string()), "{items:?}");
}

#[test]
fn runtime_402_objects_offer_their_registered_members() {
    for (source, expected) in [
        ("SURFACE value\nvalue.", &["Width", "SetPixel", "PresentRect"][..]),
        ("AUDIO value\nvalue.", &["Valid", "Volume", "Play"][..]),
        ("EVENT value\nvalue.", &["Kind", "Action", "LeftDown", "Ctrl"][..]),
        ("ERROR value\nvalue.", &["OK", "Message", "Channel"][..]),
        ("TERMINFO value\nvalue.", &["Program", "Columns", "InlineGraphics", "PixelMouse", "ClientBlit"][..]),
        ("TERMSTATE value\nvalue.", &["MarginTop", "MarginLeft", "HorizontalMargins"][..]),
        ("TERMINPUT value\nvalue.", &["Poll", "Wait", "KeyboardOn", "Release"][..]),
        ("TERMINAL value\nvalue.", &["Info", "Gfx", "Input"][..]),
        ("GFX value\nvalue.", &["Init", "Backend", "Pacing"][..]),
    ] {
        let items = complete(source);
        for member in expected {
            assert!(items.contains(&member.to_string()), "{member} missing for {source:?}: {items:?}");
        }
    }
}

/// The list is the type's own, so a member another type happens to have is not in it.
#[test]
fn an_object_offers_only_its_own_members() {
    let items = complete("TERMINPUT value\nvalue.");
    assert_eq!(items, vec!["KeyboardOff", "KeyboardOn", "MouseOff", "MouseOn", "Poll", "Release", "Wait"]);
}

#[test]
fn graphics_only_offers_session_control() {
    let items = complete("GFX value\nvalue.");
    assert_eq!(items, vec!["Backend", "Init", "Pacing", "Shutdown"]);
}

#[test]
fn event_does_not_offer_raw_masks() {
    let items = complete("EVENT value\nvalue.");
    assert!(!items.contains(&"Buttons".to_string()), "{items:?}");
    assert!(!items.contains(&"Modifiers".to_string()), "{items:?}");
    for name in ["LeftDown", "MiddleDown", "RightDown", "Shift", "Alt", "Ctrl", "Meta"] {
        assert!(items.contains(&name.to_string()), "{name} missing: {items:?}");
    }
}

#[test]
fn a_record_literal_offers_the_fields_it_has_not_named() {
    let items = complete(&format!("{RECORD}m = Member {{ "));
    assert_eq!(items, vec!["Name".to_string(), "Age".to_string(), "Home".to_string()]);

    let items = complete(&format!("{RECORD}m = Member {{ Name = \"a\", "));
    assert_eq!(items, vec!["Age".to_string(), "Home".to_string()]);
}

#[test]
fn a_record_literal_value_is_not_a_field_name() {
    let items = complete(&format!("{RECORD}m = Member {{ Age = "));
    assert!(!items.is_empty());
    assert!(!items.contains(&"Age".to_string()), "{items:?}");
}

#[test]
fn declared_types_can_be_completed() {
    let items = complete(&format!("{RECORD}Mem"));
    assert!(items.contains(&"Member".to_string()), "{items:?}");
    assert!(items.contains(&"Conference".to_string()), "{items:?}");
}

#[test]
fn nothing_is_offered_inside_a_string() {
    assert!(complete("PRINTLN \"m.").is_empty());
}

#[test]
fn signature_help_for_a_user_function() {
    let source = "DECLARE FUNCTION Total(INTEGER value, STRING name) INTEGER\nx = Total(1, ";
    assert_eq!(help(source), vec!["FUNCTION Total(INTEGER value, STRING name) INTEGER".to_string()]);
}

#[test]
fn signature_help_shows_a_var_parameter() {
    let source = "DECLARE PROCEDURE Fill(VAR INTEGER values(10))\nFill(";
    assert_eq!(help(source), vec!["PROCEDURE Fill(VAR INTEGER values(10))".to_string()]);
}

#[test]
fn signature_help_for_a_user_procedure() {
    let source = "PROCEDURE Show(STRING text)\nENDPROC\nShow(";
    assert_eq!(help(source), vec!["PROCEDURE Show(STRING text)".to_string()]);
}

#[test]
fn signature_help_for_a_routine_parameter() {
    let source = "PROCEDURE Apply(FUNCTION f(INTEGER a) INTEGER)\nENDPROC\nApply(";
    assert_eq!(help(source), vec!["PROCEDURE Apply(FUNCTION f(INTEGER a) INTEGER)".to_string()]);
}

#[test]
fn signature_help_for_a_built_in_function() {
    let source = "x = Mid(a, 1, ";
    let signatures = help(source);
    assert!(signatures.iter().any(|s| s.starts_with("MID(")), "{signatures:?}");
}

#[test]
fn signature_help_for_a_built_in_statement() {
    let source = "ANSIPOS 1, ";
    let signatures = help(source);
    assert_eq!(signatures.len(), 1);
    assert!(signatures[0].starts_with("ANSIPOS "), "{signatures:?}");
}

#[test]
fn the_argument_the_cursor_is_in_is_marked() {
    let (_, visitor) = analyze("DECLARE PROCEDURE Show(INTEGER a, INTEGER b)\n");
    let help = get_signature_help("Show(1, ", &visitor).unwrap();
    assert_eq!(help.active_parameter, Some(1));

    let signature = &help.signatures[help.active_signature.unwrap() as usize];
    let parameters = signature.parameters.as_ref().unwrap();
    let ParameterLabel::LabelOffsets([start, end]) = parameters[1].label else {
        panic!("expected offsets");
    };
    let label: Vec<char> = signature.label.chars().collect();
    let marked: String = label[start as usize..end as usize].iter().collect();
    assert_eq!(marked, "INTEGER b");
}

/// The words offered in an empty file written for that language version.
fn offered(version: u16) -> Vec<String> {
    offered_in(version, "")
}

/// The words offered in a file the workspace says is written for `version`.
fn offered_in(version: u16, source: &str) -> Vec<String> {
    let mut workspace = Workspace::default();
    workspace.compiler.get_or_insert_with(CompilerData::default).language_version = Some(version);

    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();

    let line = source.lines().last().unwrap_or("");
    get_completion(&ast, &visitor, line, source.chars().count())
        .into_iter()
        .map(|item| item.label)
        .collect()
}

#[test]
fn a_word_is_offered_from_the_version_that_gave_it_meaning() {
    let old = offered(340);
    let new = offered(400);

    for word in ["IF", "WHILE", "DECLARE"] {
        assert!(old.contains(&word.to_string()), "{word} should be offered in 340: {old:?}");
    }
    for word in ["CONST", "ENUM", "REPEAT", "TYPE", "EXIT"] {
        assert!(!old.contains(&word.to_string()), "{word} should not be offered in 340");
        assert!(new.contains(&word.to_string()), "{word} should be offered in 400");
    }
}

#[test]
fn a_source_states_which_language_it_is_written_in() {
    // The workspace says 400, but the file itself says otherwise.
    let items = offered_in(400, ";$LANGVERSION 340\n");
    assert!(items.contains(&"WHILE".to_string()), "{items:?}");
    for word in ["CONST", "ENUM", "TYPE"] {
        assert!(
            !items.contains(&word.to_string()),
            "{word} should not be offered after $LANGVERSION 340: {items:?}"
        );
    }

    // And the other way round: an old workspace with a file written for 400.
    let items = offered_in(340, ";$LANGVERSION 400\n");
    for word in ["CONST", "ENUM", "TYPE"] {
        assert!(items.contains(&word.to_string()), "{word} should be offered after $LANGVERSION 400: {items:?}");
    }
}

#[test]
fn a_built_in_statement_is_offered_from_the_version_that_added_it() {
    // WebRequest arrived in 400, MoveMsg in 340.
    let old = offered(330);
    assert!(old.contains(&"PrintLn".to_string()), "{old:?}");
    assert!(!old.contains(&"WebRequest".to_string()), "{old:?}");
    assert!(!old.contains(&"MoveMsg".to_string()), "{old:?}");

    let new = offered(400);
    assert!(new.contains(&"WebRequest".to_string()), "{new:?}");
    assert!(new.contains(&"MoveMsg".to_string()), "{new:?}");
}

#[test]
fn a_type_is_offered_from_the_version_that_named_it() {
    let first = offered(100);
    assert!(first.contains(&"INTEGER".to_string()), "{first:?}");
    for word in ["BIGSTR", "DDATE", "MSGAREAID"] {
        assert!(!first.contains(&word.to_string()), "{word} should not be offered in 100: {first:?}");
    }

    let second = offered(200);
    assert!(second.contains(&"BIGSTR".to_string()), "{second:?}");
    assert!(!second.contains(&"DDATE".to_string()), "{second:?}");

    assert!(offered(300).contains(&"DDATE".to_string()));
    assert!(offered(400).contains(&"MSGAREAID".to_string()));
}

#[test]
fn a_board_object_is_only_offered_from_400() {
    let last = offered(400);
    for word in ["Conference", "Area", "Directory", "Door"] {
        assert!(last.contains(&word.to_string()), "{word} should be offered in 400: {last:?}");
    }

    // An enum is a type from 350 on, so the objects are what 350 must not see.
    let older = offered(350);
    for word in ["Conference", "Area", "Directory", "Door"] {
        assert!(!older.contains(&word.to_string()), "{word} should not be offered in 350: {older:?}");
    }
}
