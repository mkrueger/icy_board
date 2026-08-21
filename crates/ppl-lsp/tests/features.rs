use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::Ast,
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::{document_symbol::get_document_symbols, hover::get_user_hover};
use ropey::Rope;
use tower_lsp::lsp_types::{HoverContents, SymbolKind};

fn analyze(source: &str) -> (Ast, SemanticVisitor) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor)
}

/// Hovers inside the first word of the first occurrence of `pattern`.
fn hover(source: &str, pattern: &str) -> Option<String> {
    let (ast, visitor) = analyze(source);
    let offset = source.find(pattern).unwrap_or_else(|| panic!("{pattern} not found")) + 1;
    let hover = get_user_hover(&ast, &visitor, offset)?;
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup");
    };
    Some(content.value)
}

const SOURCE: &str = r#"TYPE Member
    STRING  Name
    INTEGER Age
ENDTYPE

DECLARE PROCEDURE Show(STRING text)

Member people(10)
INTEGER count = 0

:START
count = count + 1
PRINTLN people[0].Name
Show("done")
GOTO START

PROCEDURE Show(STRING text)
    PRINTLN text
ENDPROC

FUNCTION Total(INTEGER v) INTEGER
    RETURN v + 1
ENDFUNC
"#;

#[test]
fn the_outline_lists_types_routines_and_variables() {
    let (ast, _) = analyze(SOURCE);
    let symbols = get_document_symbols(&ast, &Rope::from_str(SOURCE));

    let names: Vec<(String, SymbolKind)> = symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();
    assert_eq!(
        names,
        vec![
            ("Member".to_string(), SymbolKind::STRUCT),
            ("people".to_string(), SymbolKind::VARIABLE),
            ("count".to_string(), SymbolKind::VARIABLE),
            ("Show".to_string(), SymbolKind::METHOD),
            ("Total".to_string(), SymbolKind::FUNCTION),
        ]
    );

    let fields = symbols[0].children.as_ref().expect("the record has fields");
    assert_eq!(fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>(), vec!["Name", "Age"]);
    assert_eq!(fields[0].kind, SymbolKind::FIELD);
}

#[test]
fn the_outline_spans_the_whole_routine() {
    let (ast, _) = analyze(SOURCE);
    let symbols = get_document_symbols(&ast, &Rope::from_str(SOURCE));
    let total = symbols.iter().find(|s| s.name == "Total").unwrap();
    assert!(total.range.end.line > total.range.start.line, "{:?}", total.range);
    assert_eq!(total.selection_range.start.line, total.range.start.line);
}

#[test]
fn hover_over_a_variable_shows_its_type() {
    assert_eq!(hover(SOURCE, "count = count"), Some("```PPL\nINTEGER count\n```".to_string()));
}

#[test]
fn hover_over_an_array_shows_its_bounds() {
    assert_eq!(hover(SOURCE, "people[0]"), Some("```PPL\nMember people(10)\n```".to_string()));
}

#[test]
fn hover_over_a_routine_shows_its_signature() {
    assert_eq!(hover(SOURCE, "Show(\"done\")"), Some("```PPL\nPROCEDURE Show(STRING text)\n```".to_string()));
    assert_eq!(
        hover(SOURCE, "Total(INTEGER v)"),
        Some("```PPL\nFUNCTION Total(INTEGER v) INTEGER\n```".to_string())
    );
}

#[test]
fn hover_over_a_label_shows_it_is_one() {
    assert_eq!(hover(SOURCE, "START\ncount"), Some("```PPL\n:START\n```".to_string()));
}

#[test]
fn hover_over_a_field_shows_the_record_it_belongs_to() {
    assert_eq!(hover(SOURCE, "Name\nShow"), Some("```PPL\nSTRING Member.Name\n```".to_string()));
}

#[test]
fn hover_over_a_record_type_shows_its_fields() {
    let text = hover(SOURCE, "Member people").unwrap();
    assert!(text.contains("TYPE Member"), "{text}");
    assert!(text.contains("STRING Name"), "{text}");
    assert!(text.contains("INTEGER Age"), "{text}");
}
