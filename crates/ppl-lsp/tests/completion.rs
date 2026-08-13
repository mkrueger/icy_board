use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use icyboard_ppl::{completion::get_completion, signature_help::get_signature_help};
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
