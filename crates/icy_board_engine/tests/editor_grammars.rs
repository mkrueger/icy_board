//! The grammars an editor reads repeat the built-in names, because a parser
//! cannot ask the engine for them. This keeps every list honest.

use icy_board_engine::{
    ast::constant::BUILTIN_CONSTS,
    executable::{FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS, StatementSignature},
};

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

/// Entries of the function table that only name an operator or an internal marker.
const FUNCTIONS_NOT_IN_GRAMMAR: &[&str] = &[
    "AND", "CPAR", "DIVIDE", "END", "EQ", "EXP", "GE", "GT", "LE", "LT", "MEMBERCALL", "MEMBERREFERENCE", "MINUS", "MOD", "NE", "NOT", "OPAR", "OR", "PLUS",
    "TIMES", "UMINUS", "UPLUS",
];

/// The alternatives of one pattern of the TextMate grammar VS Code reads.
fn list_from_textmate(rule: &str) -> Vec<String> {
    let source = include_str!("../../ppl-lsp/syntaxes/ppl.tmGrammar.json");
    let start = source.find(&format!("\"{rule}\": {{")).unwrap_or_else(|| panic!("{rule} not found in the TextMate grammar"));
    let rest = &source[start..];
    let open = rest.find("\\\\b(").expect("no name list") + 4;
    let close = rest[open..].find(")\\\\b").expect("unterminated name list") + open;
    rest[open..close].split('|').map(|name| name.to_ascii_uppercase()).collect()
}

#[test]
fn vscode_grammar_knows_every_built_in() {
    let statements: Vec<String> = STATEMENT_DEFINITIONS
        .iter()
        .filter(|def| def.sig != StatementSignature::Invalid)
        .map(|def| def.name.to_ascii_uppercase())
        .collect();
    let in_grammar = list_from_textmate("builtin-statements");
    // END, LET, RETURN and STOP are keywords there, the rest has to be listed.
    let expected: Vec<String> = statements
        .iter()
        .filter(|name| !["END", "LET", "RETURN", "STOP"].contains(&name.as_str()))
        .cloned()
        .collect();
    assert_eq!(missing(&expected, &in_grammar), Vec::<String>::new(), "statements missing from the TextMate grammar");
    assert_eq!(missing(&in_grammar, &statements), Vec::<String>::new(), "the TextMate grammar names a statement the engine does not have");

    let expected: Vec<String> = FUNCTION_DEFINITIONS
        .iter()
        .map(|def| def.name.to_ascii_uppercase())
        .filter(|name| !name.starts_with('<') && !FUNCTIONS_NOT_IN_GRAMMAR.contains(&name.as_str()))
        .collect();
    let in_grammar = list_from_textmate("builtin-functions");
    assert_eq!(missing(&expected, &in_grammar), Vec::<String>::new(), "functions missing from the TextMate grammar");
    assert_eq!(missing(&in_grammar, &expected), Vec::<String>::new(), "the TextMate grammar names a function the engine does not have");

    let expected: Vec<String> = BUILTIN_CONSTS.iter().map(|c| c.name.to_ascii_uppercase()).collect();
    let in_grammar = list_from_textmate("constants");
    assert_eq!(missing(&expected, &in_grammar), Vec::<String>::new(), "constants missing from the TextMate grammar");
    assert_eq!(missing(&in_grammar, &expected), Vec::<String>::new(), "the TextMate grammar names a constant the engine does not have");
}
