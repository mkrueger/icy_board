use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use toml::Spanned;

use crate::{
    ast::{Constant, constant::NumberFormat},
    compiler::workspace::{CompilerData, Workspace},
    parser::{
        Encoding, ErrorReporter,
        lexer::{CommentType, Lexer, Token},
    },
};

#[test]
fn test_comments() {
    assert_eq!(get_token("; COMMENT"), Token::Comment(CommentType::SingleLineSemicolon, " COMMENT".to_string()));
    assert_eq!(get_token("' COMMENT"), Token::Comment(CommentType::SingleLineQuote, " COMMENT".to_string()));
    assert_eq!(get_token("* COMMENT"), Token::Comment(CommentType::SingleLineStar, " COMMENT".to_string()));
}

fn get_token(src: &str) -> Token {
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    match lex.next_token() {
        Some(t) => t,
        None => {
            panic!("Error")
        }
    }
}

fn get_spanned_token(src: &str) -> Spanned<Token> {
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    match lex.next_token() {
        Some(t) => Spanned::new(lex.span(), t),
        None => {
            panic!("Error")
        }
    }
}

fn get_token_ver(src: &str, ver: u16) -> Token {
    let mut ws = Workspace::default();
    if ws.compiler.is_none() {
        ws.compiler = Some(CompilerData::default());
    }
    ws.compiler.as_mut().unwrap().language_version = Some(ver);

    let mut lex = Lexer::new(PathBuf::from("."), &ws, src, Encoding::Utf8, Arc::new(Mutex::new(ErrorReporter::default())));
    match lex.next_token() {
        Some(t) => t,
        None => {
            panic!("Error")
        }
    }
}

#[test]
fn test_string() {
    assert_eq!(Token::Const(Constant::String(String::new())), get_token("\"\""));
    assert_eq!(Token::Const(Constant::String("\\".to_string())), get_token("\"\\\""));

    assert_eq!(Token::Const(Constant::String("\"foo\"".to_string())), get_token("\"\"\"foo\"\"\""));

    let src = "\"Hello World\" \"foo\"";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Const(Constant::String("Hello World".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Const(Constant::String("foo".to_string())), lex.next_token().unwrap());
}

#[test]
fn test_op() {
    assert_eq!(Token::Eq, get_token("=="));
    assert_eq!(Token::Eq, get_token("="));
    assert_eq!(Token::And, get_token("&&"));
    assert_eq!(Token::And, get_token("&"));
    assert_eq!(Token::Or, get_token("||"));
    assert_eq!(Token::Or, get_token("|"));
    assert_eq!(Token::Not, get_token("!"));
    //assert_eq!(Token::PoW, get_token("**"));
    assert_eq!(Token::PoW, get_token("^"));

    //assert_eq!(Token::Mul, get_token("*"));
    assert_eq!(Token::Div, get_token("/"));

    let t = get_spanned_token(" + ");
    assert_eq!(Token::Add, *t.get_ref());
    assert_eq!(1..2, t.span());

    assert_eq!(Token::Sub, get_token("-"));

    assert_eq!(Token::NotEq, get_token("<>"));
    assert_eq!(Token::NotEq, get_token("><"));
    assert_eq!(Token::NotEq, get_token("!="));
    assert_eq!(Token::Lower, get_token("<"));
    assert_eq!(Token::LowerEq, get_token("<="));
    assert_eq!(Token::LowerEq, get_token("=<"));
    assert_eq!(Token::Greater, get_token(">"));
    assert_eq!(Token::GreaterEq, get_token(">="));
    assert_eq!(Token::GreaterEq, get_token("=>"));
}

#[test]
fn test_parens() {
    assert_eq!(Token::LPar, get_token("("));
    assert_eq!(Token::RPar, get_token(")"));

    assert_eq!(Token::LBracket, get_token("["));
    assert_eq!(Token::RBracket, get_token("]"));

    assert_eq!(Token::LPar, get_token_ver("[", 340));
    assert_eq!(Token::RPar, get_token_ver("]", 340));

    assert_eq!(Token::LBrace, get_token("{"));
    assert_eq!(Token::RBrace, get_token("}"));

    assert_eq!(Token::LPar, get_token_ver("{", 340));
    assert_eq!(Token::RPar, get_token_ver("}", 340));
}

#[test]
fn test_identifier() {
    assert_eq!(Token::Identifier(unicase::Ascii::new("PRINT".to_string())), get_token("PRINT"));

    assert_eq!(Token::Identifier(unicase::Ascii::new("_".to_string())), get_token("_"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_O".to_string())), get_token("_O"));

    let src = "Hello World";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Identifier(unicase::Ascii::new("Hello".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("World".to_string())), lex.next_token().unwrap());
}

#[test]
fn test_constants() {
    assert_eq!(Token::Const(Constant::Integer(123, NumberFormat::Default)), get_token("123"));
    assert_eq!(Token::Const(Constant::Integer(100, NumberFormat::ColorCode)), get_token("@X64"));

    assert_eq!(Token::Const(Constant::Money(142)), get_token("$1.42"));
    assert_eq!(Token::Const(Constant::Integer(255, NumberFormat::Hex)), get_token("0FFh"));
    assert_eq!(Token::Const(Constant::Integer(123, NumberFormat::Dec)), get_token("123d"));
    assert_eq!(Token::Const(Constant::Integer(88, NumberFormat::Octal)), get_token("130o"));
    assert_eq!(Token::Const(Constant::Integer(8, NumberFormat::Binary)), get_token("1000b"));
    assert_eq!(Token::Const(Constant::Builtin(&crate::ast::constant::BuiltinConst::TRUE)), get_token("TRUE"));
    assert_eq!(Token::Const(Constant::Builtin(&crate::ast::constant::BuiltinConst::FALSE)), get_token("FALSE"));
    assert_eq!(Token::Const(Constant::Double(3.15)), get_token("3.15"));
    assert_eq!(Token::Const(Constant::Double(3.15)), get_token("3.15"));
    assert_eq!(Token::Const(Constant::Integer(0x0B00, NumberFormat::Hex)), get_token("0B00h"));
    assert_eq!(Token::Const(Constant::Unsigned(142_9496_7296u64)), get_token("14294967296"));
}

#[test]
fn test_no_constant() {
    assert_eq!(Token::Identifier(unicase::Ascii::new("SEC".to_string())), get_token("SEC("));
}

#[test]
fn test_errors() {
    /* PPLC takes these numbers and parses them to -1
    let src = "34877539875349573940";
    let mut lex = Lexer::new(PathBuf::from("."), src, Encoding::Utf8);
    let res = lex.next_token().unwrap();
    assert!(res.is_err());
    println!("got expected error: {res:?}");
    */
}

#[test]
fn test_eol() {
    let src = "A\nB\r\nC";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Identifier(unicase::Ascii::new("A".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Eol, lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("B".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Eol, lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("C".to_string())), lex.next_token().unwrap());
}

#[test]
fn test_colon_eol() {
    let src = "A:B:C";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Identifier(unicase::Ascii::new("A".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Eol, lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("B".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Eol, lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("C".to_string())), lex.next_token().unwrap());
}

#[test]
fn test_end_constructs() {
    assert_eq!(Token::EndSelect, get_token("EndSelect"));
    assert_eq!(Token::EndFunc, get_token("ENDFUNC"));
    assert_eq!(Token::EndProc, get_token("ENDPROC"));
}

#[test]
fn test_while() {
    assert_eq!(Token::While, get_token("WHILE"));
    assert_eq!(Token::EndWhile, get_token("ENDWHILE"));
}

#[test]
fn test_break() {
    assert_eq!(Token::Break, get_token("break"));
}

#[test]
fn test_continue() {
    assert_eq!(Token::Continue, get_token("continue"));
}

#[test]
fn test_skip() {
    let src = "Hello _\n World";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Identifier(unicase::Ascii::new("Hello".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("World".to_string())), lex.next_token().unwrap());

    let src = "Hello \\\n World";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Identifier(unicase::Ascii::new("Hello".to_string())), lex.next_token().unwrap());
    assert_eq!(Token::Identifier(unicase::Ascii::new("World".to_string())), lex.next_token().unwrap());
}
#[test]
fn test_if_then() {
    assert_eq!(Token::If, get_token("IF"));
    assert_eq!(Token::Else, get_token("ELSE"));
    assert_eq!(Token::ElseIf, get_token("ElseIf"));
    assert_eq!(Token::EndIf, get_token("EndIf"));
}

#[test]
fn test_labels() {
    assert_eq!(get_token(":label001"), Token::Label(unicase::Ascii::new("label001".to_string())));
    assert_eq!(get_token(":           label001"), Token::Label(unicase::Ascii::new("label001".to_string())));

    assert_eq!(get_token(":END"), Token::Label(unicase::Ascii::new("END".to_string())));
}

#[test]
fn test_dotdot() {
    assert_eq!(Token::DotDot, get_token(".."));

    let src = "1..";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    assert_eq!(Token::Const(Constant::Integer(1, NumberFormat::Default)), lex.next_token().unwrap());
    assert_eq!(Token::DotDot, lex.next_token().unwrap());
}

#[test]
fn test_case_number() {
    let src = "CASE 1";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    assert_eq!(Token::Case, lex.next_token().unwrap());
    assert_eq!(0..4, lex.span());

    assert_eq!(Token::Const(Constant::Integer(1, NumberFormat::Default)), lex.next_token().unwrap());
    assert_eq!(5..6, lex.span());
}

#[test]
fn test_define() {
    assert_eq!(Token::Identifier(unicase::Ascii::new("PRINT".to_string())), get_token("PRINT"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_".to_string())), get_token("_"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_O".to_string())), get_token("_O"));

    let src = ";$DEFINE FOO";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    while let Some(_token) = lex.next_token() {}
    assert_eq!(Constant::Boolean(true), *lex.get_define("FOO").unwrap());
}

#[test]
fn test_define_arithmetic() {
    assert_eq!(Token::Identifier(unicase::Ascii::new("PRINT".to_string())), get_token("PRINT"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_".to_string())), get_token("_"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_O".to_string())), get_token("_O"));

    let src = ";$DEFINE FOO=1+2";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    while let Some(_token) = lex.next_token() {}
    assert_eq!(Constant::Integer(3, NumberFormat::Default), *lex.get_define("FOO").unwrap());
}

#[test]
fn test_if() {
    assert_eq!(Token::Identifier(unicase::Ascii::new("PRINT".to_string())), get_token("PRINT"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_".to_string())), get_token("_"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_O".to_string())), get_token("_O"));

    let src = ";$IF LANGVERSION > 0\nPRIINT\n$ENDIF";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    assert_eq!(Token::Identifier(unicase::Ascii::new("PRIINT".to_string())), lex.next_token().unwrap());
}

#[test]
fn test_if2() {
    assert_eq!(Token::Identifier(unicase::Ascii::new("PRINT".to_string())), get_token("PRINT"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_".to_string())), get_token("_"));
    assert_eq!(Token::Identifier(unicase::Ascii::new("_O".to_string())), get_token("_O"));

    let src = ";$IF 1 == 2\nPRIINT\n$ENDIF";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );
    lex.next_token().unwrap();
    assert_eq!(None, lex.next_token());
}

#[test]
fn test_if_else_branch() {
    let src = ";$IF 1 == 2\nFOO\n;$ELSE\nBAR\n;$ENDIF";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    // 1) Aggregated skipped block comment
    let c1 = lex.next_token().expect("expected aggregated comment");
    match c1 {
        Token::Comment(CommentType::SingleLineSemicolon, text) => {
            assert_eq!("$IF 1 == 2\nFOO\n;$ELSE\n", text, "unexpected collected skipped text");
        }
        other => panic!("unexpected first token: {other:?}"),
    }

    // 2) BAR
    assert_eq!(Some(Token::Identifier(unicase::Ascii::new("BAR".to_string()))), lex.next_token());

    // 3) EOL after BAR
    assert_eq!(Some(Token::Eol), lex.next_token());

    // 4) $ENDIF comment
    let c2 = lex.next_token().expect("expected $ENDIF comment");
    assert_eq!(Token::Comment(CommentType::SingleLineSemicolon, "$ENDIF".to_string()), c2);

    // 5) EOF
    assert_eq!(None, lex.next_token());
}

#[test]
fn test_if_elseif_branch() {
    let src = ";$IF 0 == 1\nFOO\n;$ELSEIF 2 == 2\nBAR\n;$ELSE\nBAZ\n;$ENDIF";
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &Workspace::default(),
        src,
        Encoding::Utf8,
        Arc::new(Mutex::new(ErrorReporter::default())),
    );

    // 1) Aggregated skipped IF + ELSEIF directive line (false branch + activating directive)
    let t1 = lex.next_token().expect("expected first aggregated comment");
    match t1 {
        Token::Comment(CommentType::SingleLineSemicolon, text) => {
            assert_eq!("$IF 0 == 1\nFOO\n;$ELSEIF 2 == 2\n", text, "unexpected aggregated false IF + ELSEIF block");
        }
        other => panic!("unexpected first token: {other:?}"),
    }

    // 2) Active code: BAR
    assert_eq!(Some(Token::Identifier(unicase::Ascii::new("BAR".into()))), lex.next_token());

    // 3) EOL after BAR (depends on newline presence; BAR line ends with '\n')
    assert_eq!(Some(Token::Eol), lex.next_token(), "expected EOL after BAR");

    // 4) Skipped ELSE block aggregated (since branch already taken), closed by its ENDIF
    let t4 = lex.next_token().expect("expected aggregated ELSE skip");
    match t4 {
        Token::Comment(CommentType::SingleLineSemicolon, text) => {
            assert_eq!("$ELSE\nBAZ\n;$ENDIF", text, "unexpected aggregated ELSE block");
        }
        other => panic!("unexpected token for ELSE block: {other:?}"),
    }

    // 5) EOF
    assert_eq!(None, lex.next_token());
}

fn lex_all(src: &str) -> (Vec<Token>, Vec<String>) {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut lex = Lexer::new(PathBuf::from("."), &Workspace::default(), src, Encoding::Utf8, errors.clone());
    let mut tokens = Vec::new();
    while let Some(token) = lex.next_token() {
        tokens.push(token);
    }
    let reported = errors.lock().unwrap().errors.iter().map(|e| e.error.to_string()).collect();
    (tokens, reported)
}

/// The identifiers a source lexes to, i.e. the code that survived the preprocessor.
fn active_code(src: &str) -> Vec<String> {
    lex_all(src)
        .0
        .iter()
        .filter_map(|token| match token {
            Token::Identifier(id) => Some(id.to_string()),
            _ => None,
        })
        .collect()
}

fn assert_active(src: &str, expected: &[&str]) {
    let (tokens, errors) = lex_all(src);
    assert!(errors.is_empty(), "unexpected errors {errors:?} for:\n{src}");
    let got: Vec<String> = tokens
        .iter()
        .filter_map(|token| match token {
            Token::Identifier(id) => Some(id.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(expected, got.as_slice(), "wrong branch taken for:\n{src}");
}

#[test]
fn test_preproc_branch_selection() {
    assert_active(";$IF 1 == 1\nA\n;$ENDIF", &["A"]);
    assert_active(";$IF 1 == 2\nA\n;$ENDIF", &[]);

    assert_active(";$IF 1 == 2\nA\n;$ELSE\nB\n;$ENDIF", &["B"]);
    assert_active(";$IF 1 == 1\nA\n;$ELSE\nB\n;$ENDIF", &["A"]);

    assert_active(";$IF 1 == 2\nA\n;$ELSEIF 1 == 1\nB\n;$ELSE\nC\n;$ENDIF", &["B"]);
    // A false ELSEIF used to fall through and wrongly activate its branch.
    assert_active(";$IF 1 == 2\nA\n;$ELSEIF 1 == 3\nB\n;$ELSE\nC\n;$ENDIF", &["C"]);
    assert_active(";$IF 1 == 2\nA\n;$ELSEIF 1 == 3\nB\n;$ENDIF", &[]);

    // Only the first true branch runs.
    assert_active(";$IF 1 == 1\nA\n;$ELSEIF 1 == 1\nB\n;$ELSE\nC\n;$ENDIF", &["A"]);
    assert_active(";$IF 1 == 2\nA\n;$ELSEIF 1 == 1\nB\n;$ELSEIF 1 == 1\nC\n;$ENDIF", &["B"]);
}

#[test]
fn test_preproc_elif_is_a_synonym_for_elseif() {
    assert_active(";$IF 1 == 2\nA\n;$ELIF 1 == 1\nB\n;$ENDIF", &["B"]);
    assert_active(";$IF 1 == 2\nA\n;$ELIF 1 == 3\nB\n;$ELSE\nC\n;$ENDIF", &["C"]);
    assert_active(";$IF 1 == 1\nA\n;$ELIF 1 == 1\nB\n;$ENDIF", &["A"]);
}

#[test]
fn test_preproc_directives_are_case_insensitive() {
    assert_active(";$if 1 == 2\nA\n;$elseif 1 == 1\nB\n;$endif", &["B"]);
    assert_active(";$If 1 == 2\nA\n;$Else\nB\n;$EndIf", &["B"]);
}

#[test]
fn test_preproc_directives_allow_whitespace_after_comment_marker() {
    assert_active(";  $IF 1 == 1\nA\n;  $ENDIF", &["A"]);
    assert_active(";\t$IF 1 == 2\nA\n;\t$ELSE\nB\n;\t$ENDIF", &["B"]);

    let (tokens, errors) = lex_all(";  $DEFINE FOO=42\nX = ;#FOO");
    assert!(errors.is_empty(), "{errors:?}");
    assert!(tokens.contains(&Token::Const(Constant::Integer(42, NumberFormat::Default))), "{tokens:?}");
}

#[test]
fn test_preproc_directive_names_require_a_boundary() {
    assert_active(";$IF 1 == 1\nA\n;$ELSEWHERE\nB\n;$ENDIF", &["A", "B"]);
    assert_active(";$IF 1 == 1\nA\n;$ENDIF_EXTRA\nB\n;$ENDIF", &["A", "B"]);
    assert_active(";$IF 1 == 2\nA\n;$ELIFOO 1 == 1\nB\n;$ELSE\nC\n;$ENDIF", &["C"]);
}

#[test]
fn test_preproc_nested_blocks() {
    assert_active(";$IF 1 == 1\n;$IF 1 == 2\nA\n;$ENDIF\nB\n;$ENDIF", &["B"]);
    assert_active(";$IF 1 == 1\n;$IF 1 == 1\nA\n;$ENDIF\nB\n;$ENDIF", &["A", "B"]);
    // The whole nested block sits inside a branch that is not taken.
    assert_active(";$IF 1 == 2\n;$IF 1 == 1\nA\n;$ENDIF\nB\n;$ENDIF\nC", &["C"]);
    assert_active(";$IF 1 == 2\n;$IF 1 == 1\nA\n;$ELSE\nB\n;$ENDIF\n;$ELSE\nC\n;$ENDIF", &["C"]);
}

#[test]
fn test_preproc_unbalanced_directives_are_reported() {
    for (src, expected) in [
        (";$ELSE\nA", "$ELSE without $IF"),
        (";$ELSEIF 1 == 1\nA", "$ELSEIF without $IF"),
        (";$ELIF 1 == 1\nA", "$ELSEIF without $IF"),
        (";$ENDIF\nA", "$ENDIF without $IF"),
        (";$IF 1 == 1\nA", "Missing $ENDIF"),
        (";$IF 1 == 2\nA", "Missing $ENDIF"),
    ] {
        let (_, errors) = lex_all(src);
        assert!(errors.iter().any(|e| e == expected), "expected {expected:?}, got {errors:?} for:\n{src}");
    }
}

fn assert_preproc_error(src: &str, expected: &str) {
    let (_, errors) = lex_all(src);
    assert_eq!(vec![expected.to_string()], errors, "unexpected diagnostics for:\n{src}");
}

#[test]
fn test_preproc_malformed_conditions_have_dedicated_errors() {
    assert_preproc_error(";$IF\nA\n;$ENDIF", "Invalid pre processor expression: ''");
    assert_preproc_error(";$IF 1 ==\nA\n;$ENDIF", "Invalid pre processor expression: '1 =='");
    assert_preproc_error(";$IF 1 2\nA\n;$ENDIF", "Invalid pre processor expression: '1 2'");
    assert_preproc_error(
        ";$IF 1 == 2\nA\n;$ELSEIF\nB\n;$ENDIF",
        "Invalid pre processor expression: ''",
    );
    assert_preproc_error(
        ";$IF 1 == 2\nA\n;$ELIF 1 ==\nB\n;$ENDIF",
        "Invalid pre processor expression: '1 =='",
    );
    assert_preproc_error(
        ";$IF 1 == 1\nA\n;$ELSEIF 1 ==\nB\n;$ENDIF",
        "Invalid pre processor expression: '1 =='",
    );
    assert_preproc_error(
        ";$IF 1 == 1\nA\n;$ELSEIF 1 == 2\nB\n;$ELSEIF 1 ==\nC\n;$ENDIF",
        "Invalid pre processor expression: '1 =='",
    );
    assert_preproc_error(";$IF 1/0\nA\n;$ENDIF", "Invalid pre processor expression: '1/0'");
    assert_preproc_error(";$IF 1%0\nA\n;$ENDIF", "Invalid pre processor expression: '1%0'");
}

#[test]
fn test_preproc_malformed_defines_have_dedicated_errors() {
    assert_preproc_error(";$DEFINE", "Invalid $DEFINE directive: 'missing name'");
    assert_preproc_error(";$DEFINE = 1", "Invalid $DEFINE directive: '= 1'");
    assert_preproc_error(";$DEFINE 1FOO", "Invalid $DEFINE directive: '1FOO'");
    assert_preproc_error(";$DEFINE FOO BAR", "Invalid $DEFINE directive: 'FOO BAR'");
    assert_preproc_error(";$DEFINE FOO=", "Invalid $DEFINE directive: 'FOO='");
    assert_preproc_error(";$DEFINE FOO=1 2", "Invalid $DEFINE directive: 'FOO=1 2'");
    assert_preproc_error(";$DEFINE FOO==", "Invalid $DEFINE directive: 'FOO=='");
    assert_preproc_error(";$DEFINE FOO=1/0", "Invalid $DEFINE directive: 'FOO=1/0'");
    assert_preproc_error(";$DEFINE FOO=1%0", "Invalid $DEFINE directive: 'FOO=1%0'");
    assert_preproc_error(";$DEFINE FOO=UNKNOWN", "Invalid define value: FOO=UNKNOWN");
    assert_preproc_error(";$DEFINE FOO=\"text\"", "Invalid define value: FOO=\"text\"");
}

#[test]
fn test_preproc_define_allows_whitespace_and_boolean_values() {
    assert_active(";$DEFINE FEATURE = 1 == 1\n;$IF FEATURE\nA\n;$ENDIF", &["A"]);
    assert_active(";$DEFINE FEATURE=1=1\n;$IF FEATURE\nA\n;$ENDIF", &["A"]);
}

#[test]
fn test_preproc_define_drives_conditionals() {
    assert_active(";$DEFINE FOO\n;$IF FOO\nA\n;$ELSE\nB\n;$ENDIF", &["A"]);
    assert_active(";$DEFINE FOO=2\n;$IF FOO == 2\nA\n;$ELSE\nB\n;$ENDIF", &["A"]);
    assert_active(";$DEFINE FOO=2\n;$IF FOO == 3\nA\n;$ELSE\nB\n;$ENDIF", &["B"]);
    assert_active(";$DEFINE DEBUG_BUILD\n;$IF DEBUG_BUILD\nA\n;$ENDIF", &["A"]);
    // An undefined name is simply false rather than an error.
    assert_active(";$IF NOPE\nA\n;$ELSE\nB\n;$ENDIF", &["B"]);
}

#[test]
fn test_preproc_workspace_defines() {
    let mut workspace = Workspace::default();
    workspace.compiler = Some(CompilerData {
        language_version: Some(400),
        defines: Some(vec!["FEATURE=42".to_string(), "DEBUG_BUILD".to_string()]),
    });
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut lex = Lexer::new(
        PathBuf::from("."),
        &workspace,
        ";$IF FEATURE == 42 & DEBUG_BUILD\nA\n;$ENDIF",
        Encoding::Utf8,
        errors.clone(),
    );
    let mut identifiers = Vec::new();
    while let Some(token) = lex.next_token() {
        if let Token::Identifier(identifier) = token {
            identifiers.push(identifier.to_string());
        }
    }
    assert!(errors.lock().unwrap().errors.is_empty());
    assert_eq!(vec!["A".to_string()], identifiers);
}

#[test]
fn test_preproc_predefined_variables() {
    let (tokens, errors) = lex_all("X = ;#RUNTIME");
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        tokens.contains(&Token::Const(Constant::Integer(400, NumberFormat::Default))),
        "RUNTIME did not substitute: {tokens:?}"
    );

    let (tokens, errors) = lex_all("X = ;#LANGVERSION");
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        tokens.contains(&Token::Const(Constant::Integer(400, NumberFormat::Default))),
        "LANGVERSION did not substitute: {tokens:?}"
    );
}

#[test]
fn test_preproc_substitution_of_user_define() {
    let (tokens, errors) = lex_all(";$DEFINE FOO=41+1\nX = ;#FOO");
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        tokens.contains(&Token::Const(Constant::Integer(42, NumberFormat::Default))),
        "FOO did not substitute: {tokens:?}"
    );
}

#[test]
fn test_preproc_substitution_keeps_the_rest_of_the_line() {
    // The terminator after the name used to be swallowed, which ate the following token.
    let (tokens, _) = lex_all(";$DEFINE FOO=1\nA ;#FOO B");
    let idents: Vec<String> = tokens
        .iter()
        .filter_map(|token| match token {
            Token::Identifier(id) => Some(id.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(vec!["A".to_string(), "B".to_string()], idents, "{tokens:?}");
}

#[test]
fn test_preproc_undefined_substitution_does_not_truncate() {
    // An unknown token used to report end of file, silently dropping everything after it.
    let (tokens, errors) = lex_all("A\n;#NOPE\nB");
    assert!(
        errors.iter().any(|e| e == "Undefined pre processor token (NOPE)"),
        "expected an undefined token error, got {errors:?}"
    );
    let idents: Vec<String> = tokens
        .iter()
        .filter_map(|token| match token {
            Token::Identifier(id) => Some(id.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(vec!["A".to_string(), "B".to_string()], idents, "code after the bad token was dropped");
}

#[test]
fn test_preproc_unknown_directive_is_a_comment() {
    assert_active(";$SOMETHINGELSE\nA", &["A"]);
    assert_eq!(vec!["A".to_string()], active_code(";$SOMETHINGELSE\nA"));
}

/// Renders the whole token stream so a refactoring cannot quietly change which
/// tokens a skipped region turns into.
fn token_stream(src: &str) -> String {
    lex_all(src)
        .0
        .iter()
        .map(|token| match token {
            Token::Comment(marker, text) => format!("comment({marker}{})", text.escape_debug()),
            Token::Identifier(id) => format!("id({id})"),
            Token::Eol => "eol".to_string(),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn test_preproc_skipped_region_token_stream() {
    // An active branch lexes its code and leaves the ENDIF as its own comment.
    assert_eq!("id(A) eol comment(;$ENDIF)", token_stream(";$IF 1 == 1\nA\n;$ENDIF"));

    // A skipped region is one comment, closed by the ENDIF it consumed.
    assert_eq!("comment(;$IF 1 == 2\\nA\\n;$ENDIF)", token_stream(";$IF 1 == 2\nA\n;$ENDIF"));

    // An activating ELSE closes the comment so the code behind it is lexed.
    assert_eq!(
        "comment(;$IF 1 == 2\\nA\\n;$ELSE\\n) id(B) eol comment(;$ENDIF)",
        token_stream(";$IF 1 == 2\nA\n;$ELSE\nB\n;$ENDIF")
    );

    // A trailing skipped branch runs to the end of the block.
    assert_eq!("id(A) eol comment(;$ELSE\\nB\\n;$ENDIF)", token_stream(";$IF 1 == 1\nA\n;$ELSE\nB\n;$ENDIF"));

    // A false ELSEIF keeps collecting until the branch that does activate.
    assert_eq!(
        "comment(;$IF 1 == 2\\nA\\n;$ELSEIF 1 == 3\\nB\\n;$ELSE\\n) id(C) eol comment(;$ENDIF)",
        token_stream(";$IF 1 == 2\nA\n;$ELSEIF 1 == 3\nB\n;$ELSE\nC\n;$ENDIF")
    );

    // A nested block inside a skipped branch is absorbed whole.
    assert_eq!(
        "comment(;$IF 1 == 2\\nA\\n;$IF 1 == 1\\nB\\n;$ENDIF\\nC\\n;$ENDIF\\n) id(D)",
        token_stream(";$IF 1 == 2\nA\n;$IF 1 == 1\nB\n;$ENDIF\nC\n;$ENDIF\nD")
    );

    // Code after the block keeps its own tokens.
    assert_eq!("comment(;$IF 1 == 2\\nA\\n;$ENDIF\\n) id(B)", token_stream(";$IF 1 == 2\nA\n;$ENDIF\nB"));
}

#[test]
fn test_preproc_comment_tokens_reproduce_the_source() {
    // The marker belongs to the CommentType, so printing a comment back must not
    // double it. Skipped regions used to render as ";;$IF ...". Only regions that
    // are skipped keep their directive lines; an active directive emits no token.
    for src in [
        ";$IF 1 == 2\nA\n;$ENDIF",
        ";$IF 1 == 2\nA\n;$ELSE\nB\n;$ENDIF",
        ";$IF 1 == 2\nA\n;$IF 1 == 1\nB\n;$ENDIF\nC\n;$ENDIF",
    ] {
        let rendered: String = lex_all(src)
            .0
            .iter()
            .map(|token| match token {
                Token::Comment(marker, text) => format!("{marker}{text}"),
                Token::Identifier(id) => id.to_string(),
                Token::Eol => "\n".to_string(),
                other => panic!("unexpected token {other:?} for {src:?}"),
            })
            .collect();
        assert_eq!(src, rendered, "comment tokens did not reproduce the source");
    }
}

#[test]
fn test_preproc_skipped_region_preserves_crlf() {
    let src = ";$IF 1 == 2\r\nA\r\n;$ENDIF";
    let rendered: String = lex_all(src)
        .0
        .iter()
        .map(|token| match token {
            Token::Comment(marker, text) => format!("{marker}{text}"),
            other => panic!("unexpected token {other:?}"),
        })
        .collect();
    assert_eq!(src, rendered);
}

#[test]
fn test_preproc_skipped_region_preserves_comment_marker() {
    for src in ["'$IF 1 == 2\nA\n'$ENDIF", "*$IF 1 == 2\nA\n*$ENDIF"] {
        let rendered: String = lex_all(src)
            .0
            .iter()
            .map(|token| match token {
                Token::Comment(marker, text) => format!("{marker}{text}"),
                other => panic!("unexpected token {other:?}"),
            })
            .collect();
        assert_eq!(src, rendered);
    }
}

#[test]
fn test_preproc_skipped_region_reports_only_one_missing_endif() {
    let (_, errors) = lex_all(";$IF 1 == 2\nA");
    assert_eq!(vec!["Missing $ENDIF".to_string()], errors);
}
