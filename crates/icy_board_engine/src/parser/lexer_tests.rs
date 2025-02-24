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
    lex.next_token().unwrap();
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
