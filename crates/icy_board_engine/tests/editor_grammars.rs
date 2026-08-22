//! The grammars an editor reads repeat the built-in names, because a parser
//! cannot ask the engine for them. This keeps every list honest.

use icy_board_engine::{
    ast::constant::BUILTIN_CONSTS,
    executable::{FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS, StatementSignature},
    parser::lexer::KEYWORDS,
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
    "MEMBERCALL",
    "ONERROR",
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
    let start = source
        .find(&format!("const {name} = ["))
        .unwrap_or_else(|| panic!("{name} not found in grammar.js"));
    let rest = &source[start..];
    let end = rest.find("];").expect("unterminated list");
    rest[..end].split('\'').skip(1).step_by(2).map(|entry| entry.to_ascii_uppercase()).collect()
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
    assert_eq!(
        missing(&in_grammar, &expected),
        Vec::<String>::new(),
        "grammar.js names a statement the engine does not have"
    );
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
    assert_eq!(
        missing(&in_grammar, &expected),
        Vec::<String>::new(),
        "grammar.js names a constant the engine does not have"
    );
}

/// Entries of the function table that only name an operator or an internal marker.
const FUNCTIONS_NOT_IN_GRAMMAR: &[&str] = &[
    "AND",
    "CPAR",
    "DIVIDE",
    "END",
    "EQ",
    "EXP",
    "GE",
    "GT",
    "LE",
    "LT",
    "MEMBERCALL",
    "MEMBERREFERENCE",
    "MINUS",
    "MOD",
    "NE",
    "NOT",
    "OPAR",
    "OR",
    "PLUS",
    "TIMES",
    "UMINUS",
    "UPLUS",
];

/// The alternatives of one pattern of the TextMate grammar VS Code reads.
fn list_from_textmate(rule: &str) -> Vec<String> {
    let source = include_str!("../../../editors/vscode/syntaxes/ppl.tmGrammar.json");
    let start = source
        .find(&format!("\"{rule}\": {{"))
        .unwrap_or_else(|| panic!("{rule} not found in the TextMate grammar"));
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
    assert_eq!(
        missing(&expected, &in_grammar),
        Vec::<String>::new(),
        "statements missing from the TextMate grammar"
    );
    assert_eq!(
        missing(&in_grammar, &statements),
        Vec::<String>::new(),
        "the TextMate grammar names a statement the engine does not have"
    );

    let expected: Vec<String> = FUNCTION_DEFINITIONS
        .iter()
        .map(|def| def.name.to_ascii_uppercase())
        .filter(|name| !name.starts_with('<') && !FUNCTIONS_NOT_IN_GRAMMAR.contains(&name.as_str()))
        .collect();
    let in_grammar = [list_from_textmate("builtin-functions"), list_from_textmate("terminal-info")].concat();
    assert_eq!(
        missing(&expected, &in_grammar),
        Vec::<String>::new(),
        "functions missing from the TextMate grammar"
    );
    assert_eq!(
        missing(&in_grammar, &expected),
        Vec::<String>::new(),
        "the TextMate grammar names a function the engine does not have"
    );

    let expected: Vec<String> = BUILTIN_CONSTS.iter().map(|c| c.name.to_ascii_uppercase()).collect();
    let in_grammar = list_from_textmate("constants");
    assert_eq!(
        missing(&expected, &in_grammar),
        Vec::<String>::new(),
        "constants missing from the TextMate grammar"
    );
    assert_eq!(
        missing(&in_grammar, &expected),
        Vec::<String>::new(),
        "the TextMate grammar names a constant the engine does not have"
    );
}

/// Words a grammar colours as a keyword that the lexer does not reserve: THEN, DO,
/// TO, STEP and VAR are read as plain names, TRUE and FALSE are constants, END, EXIT
/// and STOP are statements, ON, ERROR and OFF only mean something next to each other
/// in ON ERROR, and the engine reads END FUNCTION as two tokens rather than as one word.
const COLOURED_BUT_NOT_RESERVED: &[&str] = &[
    "DO",
    "END",
    "ENDFUNCTION",
    "ENDPROCEDURE",
    "ERROR",
    "EXIT",
    "FALSE",
    "OFF",
    "ON",
    "STEP",
    "STOP",
    "THEN",
    "TO",
    "TRUE",
    "VAR",
];

fn reserved_words() -> Vec<String> {
    KEYWORDS.iter().map(|keyword| keyword.name.to_ascii_uppercase()).collect()
}

/// The words `kw('IF')` and `endKw('IF')` spell, the latter standing for `ENDIF`.
fn keywords_from_grammar() -> Vec<String> {
    let source = include_str!("../../tree-sitter-ppl/grammar.js");
    let mut names = Vec::new();

    for (call, prefix) in [("kw('", ""), ("endKw('", "END")] {
        let mut rest = source;
        while let Some(start) = rest.find(call) {
            rest = &rest[start + call.len()..];
            let end = rest.find('\'').expect("unterminated keyword in grammar.js");
            names.push(format!("{prefix}{}", rest[..end].to_ascii_uppercase()));
            rest = &rest[end..];
        }
    }
    names
}

/// Every alternative of the keyword patterns, `END\s+(IF|...)` included.
fn keywords_from_textmate() -> Vec<String> {
    let source = include_str!("../../../editors/vscode/syntaxes/ppl.tmGrammar.json");
    let start = source.find("\"keywords\": {").expect("no keyword rule in the TextMate grammar");
    let section = &source[start..];
    let section = &section[..section.find("\"types\":").expect("the keyword rule does not end")];

    let mut names = Vec::new();
    let mut idx = 0;
    while let Some(open) = section[idx..].find('(') {
        let open = idx + open + 1;
        let Some(close) = section[open..].find(')') else {
            break;
        };
        let close = open + close;
        idx = close + 1;

        let group = &section[open..close];
        if group.is_empty() || !group.chars().all(|c| c.is_ascii_alphabetic() || c == '|') {
            continue;
        }
        let prefix = if section[..open].ends_with("\\\\s+(") { "END" } else { "" };
        names.extend(group.split('|').map(|name| format!("{prefix}{}", name.to_ascii_uppercase())));
    }
    names
}

#[test]
fn grammar_reserves_every_keyword() {
    let in_grammar = keywords_from_grammar();
    assert_eq!(
        missing(&reserved_words(), &in_grammar),
        Vec::<String>::new(),
        "keywords missing from grammar.js"
    );

    let allowed: Vec<String> = reserved_words()
        .into_iter()
        .chain(COLOURED_BUT_NOT_RESERVED.iter().map(|name| name.to_string()))
        .collect();
    assert_eq!(
        missing(&in_grammar, &allowed),
        Vec::<String>::new(),
        "grammar.js reserves a word the lexer does not"
    );
}

#[test]
fn vscode_grammar_reserves_every_keyword() {
    let in_grammar = keywords_from_textmate();
    assert_eq!(
        missing(&reserved_words(), &in_grammar),
        Vec::<String>::new(),
        "keywords missing from the TextMate grammar"
    );

    let allowed: Vec<String> = reserved_words()
        .into_iter()
        .chain(COLOURED_BUT_NOT_RESERVED.iter().map(|name| name.to_string()))
        .collect();
    assert_eq!(
        missing(&in_grammar, &allowed),
        Vec::<String>::new(),
        "the TextMate grammar colours a word the lexer does not reserve"
    );
}

/// The words of the `END\s*(...)` group of the pattern that follows `marker`.
fn block_ends_from_language_configuration(marker: &str) -> Vec<String> {
    let source = include_str!("../../../editors/vscode/language-configuration.json");
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("{marker} not found in the language configuration"));
    let rest = &source[start..];

    let group = "END\\\\s*(";
    let open = rest.find(group).expect("no END group") + group.len();
    let close = rest[open..].find(')').expect("unterminated END group") + open;
    rest[open..close].split('|').map(|name| name.to_ascii_uppercase()).collect()
}

/// Folding and indenting stop at the word that closes a block, so every ENDx the
/// lexer knows has to be one the editor recognises.
#[test]
fn the_language_configuration_closes_every_block() {
    let expected: Vec<String> = KEYWORDS
        .iter()
        .filter_map(|keyword| keyword.name.strip_prefix("end"))
        .map(|word| word.to_ascii_uppercase())
        .collect();
    assert!(expected.len() > 5, "the keywords no longer spell their block ends");

    for (rule, marker) in [("folding", "\"end\":"), ("indentation", "\"decreaseIndentPattern\"")] {
        assert_eq!(
            missing(&expected, &block_ends_from_language_configuration(marker)),
            Vec::<String>::new(),
            "block ends missing from the {rule} rule"
        );
    }
}
