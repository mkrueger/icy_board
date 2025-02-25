use chumsky::prelude::*;
use logos::Logos;
use thiserror::Error;

use crate::Res;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Parse Error")]
    ParserError,

    #[error("Parse Token Error")]
    ParseTokenError,
}

#[derive(Debug, Logos, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    #[token("&")]
    And,
    #[token("|")]
    Or,
    #[token("!")]
    Not,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    #[regex(r#""[^"]*""#, |lex| lex.slice().to_owned().trim_matches('"').to_string())]
    #[regex(r#"[^"&|!()\s][^"&|!()]*"#, |lex| lex.slice().trim_ascii().to_owned(), priority=3)]
    Identifier(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternExpr {
    Match(String),

    Not(Box<PatternExpr>),

    And(Box<PatternExpr>, Box<PatternExpr>),
    Or(Box<PatternExpr>, Box<PatternExpr>),
}

impl PatternExpr {
    pub fn parse(input: &str) -> Res<Self> {
        let lexer = Token::lexer(input);
        let mut tokens = vec![];
        for (token, _span) in lexer.spanned() {
            match token {
                Ok(ok) => tokens.push(ok),
                Err(_) => return Err(Box::new(ParseError::ParseTokenError)),
            }
        }
        match parser().parse(tokens) {
            Ok(ok) => Ok(ok),
            Err(_err) => Err(Box::new(ParseError::ParserError)),
        }
    }

    pub fn to_regex(&self) -> String {
        match self {
            PatternExpr::Match(id) => id.clone(),
            PatternExpr::Not(pattern_expr) => format!("!({})", pattern_expr.to_regex()),
            PatternExpr::And(pattern_expr, pattern_expr1) => format!("({}|{})", pattern_expr.to_regex(), pattern_expr1.to_regex()),
            PatternExpr::Or(pattern_expr, pattern_expr1) => format!("({}|{})", pattern_expr.to_regex(), pattern_expr1.to_regex()),
        }
    }
}

#[allow(clippy::let_and_return)]
fn parser() -> impl Parser<Token, PatternExpr, Error = Simple<Token>> {
    recursive(|p| {
        let atom = {
            let parenthesized = p.clone().delimited_by(just(Token::LParen), just(Token::RParen));

            let identifier = select! {
                Token::Identifier(n) => PatternExpr::Match(n),
            };

            parenthesized.or(identifier)
        };
        let unary = just(Token::Not)
            .repeated()
            .then(atom)
            .foldr(|_op, rhs: PatternExpr| PatternExpr::Not(Box::new(rhs)));
        let binary = unary
            .clone()
            .then(just(Token::And).or(just(Token::Or)).then(unary).repeated())
            .foldl(|lhs, (op, rhs)| match op {
                Token::And => PatternExpr::And(Box::new(lhs), Box::new(rhs)),
                Token::Or => PatternExpr::Or(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            });
        binary
    })
    .then_ignore(end())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_identifier() {
        let input = r#"FOO"#;
        let expr = PatternExpr::parse(input).unwrap();
        assert_eq!(expr, PatternExpr::Match("FOO".to_owned()));
    }

    #[test]
    fn test_identifier2() {
        let input = r#"  "FOO BAR"  "#;
        let expr = PatternExpr::parse(input).unwrap();
        assert_eq!(expr, PatternExpr::Match("FOO BAR".to_owned()));
    }

    #[test]
    fn test_identifier3() {
        let input = r#"FOO BAR"#;
        let expr = PatternExpr::parse(input).unwrap();
        assert_eq!(expr, PatternExpr::Match("FOO BAR".to_owned()));
    }

    #[test]
    fn test_parser() {
        let input = r#"(a & b) | c"#;
        let expr = PatternExpr::parse(input).unwrap();
        assert_eq!(
            expr,
            PatternExpr::Or(
                Box::new(PatternExpr::And(
                    Box::new(PatternExpr::Match("a".to_owned())),
                    Box::new(PatternExpr::Match("b".to_owned()))
                )),
                Box::new(PatternExpr::Match("c".to_owned()))
            )
        );
    }
}
