use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::Ast,
    compiler::workspace::{CompilerData, Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::semantic_tokens::get_semantic_tokens;
use ropey::Rope;

const KEYWORD: u32 = 0;
const VARIABLE: u32 = 4;
const PARAMETER: u32 = 5;
const ENUM: u32 = 8;
const ENUM_MEMBER: u32 = 9;
const READONLY: u32 = 1 << 2;

fn analyze(source: &str, version: u16) -> (Ast, SemanticVisitor, Workspace) {
    let mut workspace = Workspace::default();
    workspace.compiler = Some(CompilerData {
        language_version: Some(version),
        defines: None,
    });
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor, workspace)
}

fn decoded(source: &str, version: u16) -> Vec<(String, u32, u32)> {
    let (ast, visitor, workspace) = analyze(source, version);
    let rope = Rope::from_str(source);
    let tokens = get_semantic_tokens(&ast, &visitor, &rope, source, &workspace);
    let (mut line, mut start) = (0, 0);

    tokens
        .into_iter()
        .map(|token| {
            line += token.delta_line;
            start = if token.delta_line == 0 {
                start + token.delta_start
            } else {
                token.delta_start
            };
            let text: String = source.lines().nth(line as usize).unwrap()[start as usize..(start + token.length) as usize].to_string();
            (text, token.token_type, token.token_modifiers_bitset)
        })
        .collect()
}

#[test]
fn enums_and_constants_have_semantic_kinds() {
    let source = "ENUM Color\n  Red\nENDENUM\nCONST Color Favorite = Color.Red\nColor current = Favorite\n";
    let tokens = decoded(source, 350);

    assert!(tokens.contains(&("ENUM".to_string(), KEYWORD, 0)), "{tokens:?}");
    assert!(tokens.contains(&("Color".to_string(), ENUM, 1)), "{tokens:?}");
    assert!(tokens.contains(&("Red".to_string(), ENUM_MEMBER, 1 | READONLY)), "{tokens:?}");
    assert!(tokens.contains(&("CONST".to_string(), KEYWORD, 0)), "{tokens:?}");
    assert!(tokens.contains(&("Favorite".to_string(), VARIABLE, 1 | READONLY)), "{tokens:?}");
    assert!(tokens.contains(&("Red".to_string(), ENUM_MEMBER, 0)), "{tokens:?}");
}

#[test]
fn a_word_is_a_keyword_only_in_a_version_that_reserves_it() {
    let source = "CONST INTEGER Answer = 42\n";
    let modern = decoded(source, 350);
    let legacy = decoded(source, 340);

    assert!(modern.contains(&("CONST".to_string(), KEYWORD, 0)), "{modern:?}");
    assert!(
        !legacy.iter().any(|(text, token_type, _)| text == "CONST" && *token_type == KEYWORD),
        "{legacy:?}"
    );
}

#[test]
fn a_routine_parameter_is_not_just_a_variable() {
    let source = "PROCEDURE Show(INTEGER value)\n  PRINTLN value\nENDPROC\n";
    let tokens = decoded(source, 350);
    assert!(tokens.contains(&("value".to_string(), PARAMETER, 1)), "{tokens:?}");
}

#[test]
fn token_lengths_are_utf16_code_units() {
    let source = "; 😀 note\nPRINTLN \"😀\"\n";
    let (ast, visitor, workspace) = analyze(source, 350);
    let tokens = get_semantic_tokens(&ast, &visitor, &Rope::from_str(source), source, &workspace);

    let comment = tokens.iter().find(|token| token.token_type == 1).expect("comment token");
    let string = tokens.iter().find(|token| token.token_type == 2).expect("string token");
    assert_eq!(comment.length, 9);
    assert_eq!(string.length, 4);
}
