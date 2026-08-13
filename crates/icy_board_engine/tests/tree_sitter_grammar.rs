//! The tree-sitter grammar repeats the built-in names, because a parser cannot
//! ask the engine for them. This keeps both lists honest.

use icy_board_engine::{ast::constant::BUILTIN_CONSTS, executable::STATEMENT_DEFINITIONS};

/// Names the grammar spells as a keyword or that only exist as an opcode.
const STATEMENTS_NOT_IN_GRAMMAR: &[&str] = &[
    "BEGIN",
    "DECLARE",
    "END",
    "FEND",
    "FUNCTION",
    "GOSUB",
    "GOTO",
    "IF",
    "LET",
    "PCALL",
    "PLACEHOLDER",
    "PROCEDURE",
    "RETURN",
    "STATIC",
];

/// TRUE and FALSE are literals of their own, the rest already have a token.
const CONSTANTS_NOT_IN_GRAMMAR: &[&str] = &["FALSE", "LANG", "NEWLINE", "SEC", "TRUE"];

fn list_from_grammar(name: &str) -> Vec<String> {
    let source = include_str!("../../tree-sitter-ppl/grammar.js");
    let start = source.find(&format!("const {name} = [")).unwrap_or_else(|| panic!("{name} not found in grammar.js"));
    let rest = &source[start..];
    let end = rest.find("];").expect("unterminated list");
    rest[..end]
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(|entry| entry.to_ascii_uppercase())
        .collect()
}

fn missing(expected: &[String], actual: &[String]) -> Vec<String> {
    expected.iter().filter(|name| !actual.contains(name)).cloned().collect()
}

#[test]
fn grammar_knows_every_built_in_statement() {
    let in_grammar = list_from_grammar("BUILTIN_STATEMENTS");
    let expected: Vec<String> = STATEMENT_DEFINITIONS
        .iter()
        .map(|def| def.name.to_ascii_uppercase())
        .filter(|name| !STATEMENTS_NOT_IN_GRAMMAR.contains(&name.as_str()))
        .collect();

    assert_eq!(missing(&expected, &in_grammar), Vec::<String>::new(), "statements missing from grammar.js");
    assert_eq!(missing(&in_grammar, &expected), Vec::<String>::new(), "grammar.js names a statement the engine does not have");
}

#[test]
fn grammar_knows_every_built_in_constant() {
    let in_grammar = list_from_grammar("BUILTIN_CONSTANTS");
    let expected: Vec<String> = BUILTIN_CONSTS
        .iter()
        .map(|c| c.name.to_ascii_uppercase())
        .filter(|name| !CONSTANTS_NOT_IN_GRAMMAR.contains(&name.as_str()))
        .collect();

    assert_eq!(missing(&expected, &in_grammar), Vec::<String>::new(), "constants missing from grammar.js");
    assert_eq!(missing(&in_grammar, &expected), Vec::<String>::new(), "grammar.js names a constant the engine does not have");
}
