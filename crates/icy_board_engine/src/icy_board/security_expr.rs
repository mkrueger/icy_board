use std::{fmt::Display, iter::Peekable};

use chrono::{NaiveTime, Timelike};
use logos::{Lexer, Logos};

use super::state::Session;
use crate::Res;

#[derive(Copy, Clone)]
pub enum UnaryOp {
    Not,
}
impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

#[derive(Copy, Clone)]
pub enum BinaryOp {
    And,
    Or,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}
impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::And => write!(f, "&"),
            BinaryOp::Or => write!(f, "|"),
            BinaryOp::Equal => write!(f, "="),
            BinaryOp::NotEqual => write!(f, "!="),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::GreaterEqual => write!(f, ">="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::LessEqual => write!(f, "<="),
        }
    }
}

#[derive(PartialEq, Clone, Eq, Debug)]
pub enum Value {
    Bool(bool),
    Integer(i64),
    String(String),
    Time(NaiveTime),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Time(t) => write!(f, "{:02}:{:02}:{:02}", t.hour(), t.minute(), t.second()),
        }
    }
}

#[derive(Clone)]
pub enum SecurityExpression {
    UnaryExpression(UnaryOp, Box<SecurityExpression>),
    BinaryExpression(BinaryOp, Box<SecurityExpression>, Box<SecurityExpression>),
    Call(String, Vec<SecurityExpression>),
    Constant(Value),
    Parens(Box<SecurityExpression>),
}

impl Display for SecurityExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityExpression::UnaryExpression(op, expr) => write!(f, "{}{}", op, expr),
            SecurityExpression::BinaryExpression(op, left, right) => write!(f, "{} {} {}", left, op, right),
            SecurityExpression::Call(name, args) => write!(f, "{}({})", name, args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")),
            SecurityExpression::Constant(value) => write!(f, "{}", value),
            SecurityExpression::Parens(expr) => write!(f, "({})", expr),
        }
    }
}

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
enum Token {
    // Tokens can be literal strings, of any length.
    #[token("true")]
    True,
    #[token("false")]
    False,

    #[token("!")]
    Not,

    #[token("(")]
    LPar,
    #[token(")")]
    RPar,
    #[token(",")]
    Comma,

    #[token("&")]
    And,
    #[token("|")]
    Or,
    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,

    #[token("<")]
    LT,
    #[token("<=")]
    LTE,

    #[token(">")]
    GT,
    #[token(">=")]
    GTE,

    #[regex("\"[_a-zA-Z0-9]+\"", |lex| lex.slice().to_string())]
    String(String),

    #[regex("[_a-zA-Z]+", |lex| lex.slice().to_string())]
    Text(String),

    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    Integer(i64),

    #[regex("\\d\\d:\\d\\d", |lex| lex.slice().to_string())]
    Time(String),
}

impl SecurityExpression {
    pub fn eval(&self, session: &Session) -> Res<Value> {
        match self {
            SecurityExpression::UnaryExpression(op, expr) => match op {
                UnaryOp::Not => match expr.eval(session)? {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    expr => Err(format!("Invalid operand for NOT operator {}", expr).into()),
                },
            },
            SecurityExpression::BinaryExpression(op, left, right) => match op {
                BinaryOp::And => match (left.eval(session)?, right.eval(session)?) {
                    (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l && r)),
                    (l, r) => Err(format!("Invalid operands for AND operator {} {}", l, r).into()),
                },
                BinaryOp::Or => match (left.eval(session)?, right.eval(session)?) {
                    (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l || r)),
                    (l, r) => Err(format!("Invalid operands for AND operator {} {}", l, r).into()),
                },
                BinaryOp::Equal => {
                    let l = left.eval(session)?;
                    let r = right.eval(session)?;
                    Ok(Value::Bool(l == r))
                }
                BinaryOp::NotEqual => {
                    let l = left.eval(session)?;
                    let r = right.eval(session)?;
                    Ok(Value::Bool(l != r))
                }
                BinaryOp::Greater => {
                    let l = left.eval(session)?;
                    let r = right.eval(session)?;
                    match (l, r) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(Value::Bool(l > r)),
                        (Value::Time(l), Value::Time(r)) => Ok(Value::Bool(l > r)),
                        (l, r) => Err(format!("Invalid operands for > operator {} {}", l, r).into()),
                    }
                }
                BinaryOp::GreaterEqual => {
                    let l = left.eval(session)?;
                    let r = right.eval(session)?;
                    match (l, r) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(Value::Bool(l >= r)),
                        (Value::Time(l), Value::Time(r)) => Ok(Value::Bool(l >= r)),
                        (l, r) => Err(format!("Invalid operands for >= operator {} {}", l, r).into()),
                    }
                }
                BinaryOp::Less => {
                    let l = left.eval(session)?;
                    let r = right.eval(session)?;
                    match (l, r) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(Value::Bool(l < r)),
                        (Value::Time(l), Value::Time(r)) => Ok(Value::Bool(l < r)),
                        (l, r) => Err(format!("Invalid operands for < operator {} {}", l, r).into()),
                    }
                }
                BinaryOp::LessEqual => {
                    let l = left.eval(session)?;
                    let r = right.eval(session)?;
                    match (l, r) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(Value::Bool(l <= r)),
                        (Value::Time(l), Value::Time(r)) => Ok(Value::Bool(l <= r)),
                        (l, r) => Err(format!("Invalid operands for <= operator {} {}", l, r).into()),
                    }
                }
            },
            SecurityExpression::Call(name, args) => match name.to_uppercase().as_str() {
                "U_SEC" => Ok(Value::Integer(session.cur_security as i64)),
                "U_AGE" => {
                    let age = if let Some(user) = &session.current_user {
                        chrono::Utc::now().years_since(user.birth_date).unwrap_or(0)
                    } else {
                        0
                    };
                    Ok(Value::Integer(age as i64))
                }
                "U_GROUP" => {
                    if let Value::String(group) = args[0].eval(session)? {
                        Ok(Value::Bool(session.cur_groups.contains(&group)))
                    } else {
                        Err("Invalid argument for U_GROUP".into())
                    }
                }
                _ => Err(format!("Invalid function {name}").into()),
            },
            SecurityExpression::Constant(constant) => Ok(constant.clone()),
            SecurityExpression::Parens(expr) => expr.eval(session),
        }
    }

    pub fn parse(input: &str) -> Res<SecurityExpression> {
        let mut a: Peekable<Lexer<Token>> = Token::lexer(input).peekable();
        Ok(bool_oper(&mut a)?)
    }
}

fn bool_oper(lexer: &mut Peekable<Lexer<Token>>) -> Res<SecurityExpression> {
    let left = eq_oper(lexer)?;
    let Some(next) = lexer.peek() else {
        return Ok(left);
    };
    match next {
        Ok(Token::And) => {
            lexer.next();
            let right = bool_oper(lexer)?;
            Ok(SecurityExpression::BinaryExpression(BinaryOp::And, Box::new(left), Box::new(right)))
        }
        Ok(Token::Or) => {
            lexer.next();
            let right = bool_oper(lexer)?;
            Ok(SecurityExpression::BinaryExpression(BinaryOp::Or, Box::new(left), Box::new(right)))
        }
        _ => Ok(left),
    }
}

fn eq_oper(lexer: &mut Peekable<Lexer<Token>>) -> Res<SecurityExpression> {
    let left = factor(lexer)?;
    let Some(next) = lexer.peek() else {
        return Ok(left);
    };

    match next {
        Ok(Token::LT) => {
            lexer.next();
            let right = factor(lexer)?;
            Ok(SecurityExpression::BinaryExpression(BinaryOp::Less, Box::new(left), Box::new(right)))
        }
        Ok(Token::LTE) => {
            lexer.next();
            let right = factor(lexer)?;
            Ok(SecurityExpression::BinaryExpression(BinaryOp::LessEqual, Box::new(left), Box::new(right)))
        }
        Ok(Token::GT) => {
            lexer.next();
            let right = factor(lexer)?;
            Ok(SecurityExpression::BinaryExpression(BinaryOp::Greater, Box::new(left), Box::new(right)))
        }
        Ok(Token::GTE) => {
            lexer.next();
            let right = factor(lexer)?;
            Ok(SecurityExpression::BinaryExpression(BinaryOp::GreaterEqual, Box::new(left), Box::new(right)))
        }
        _ => Ok(left),
    }
}

fn factor(lexer: &mut Peekable<Lexer<Token>>) -> Res<SecurityExpression> {
    let Some(token) = lexer.next() else {
        return Err("Unexpected end of input".into());
    };
    let Ok(token) = token else {
        return Err("Unexpected token".into());
    };

    let r = match token {
        Token::Not => {
            let expr = factor(lexer)?;
            SecurityExpression::UnaryExpression(UnaryOp::Not, Box::new(expr))
        }
        Token::LPar => {
            let expr = bool_oper(lexer)?;
            let rpar = lexer.next();
            if rpar != Some(Ok(Token::RPar)) {
                return Err("Expected ')'".into());
            }
            SecurityExpression::Parens(Box::new(expr))
        }
        Token::True => SecurityExpression::Constant(Value::Bool(true)),
        Token::False => SecurityExpression::Constant(Value::Bool(false)),
        Token::String(s) => {
            let mut s = s;
            s.pop();
            s.remove(0);
            SecurityExpression::Constant(Value::String(s))
        }
        Token::Integer(i) => SecurityExpression::Constant(Value::Integer(i)),
        Token::Time(t) => SecurityExpression::Constant(Value::Time(NaiveTime::parse_from_str(&t, "%H:%M").unwrap())),
        Token::Text(name) => {
            let lpar = lexer.next();
            if lpar != Some(Ok(Token::LPar)) {
                return Err("Expected '('".into());
            }
            let mut args = Vec::new();
            loop {
                if let Some(Ok(Token::RPar)) = lexer.peek() {
                    lexer.next();
                    break;
                }
                let arg = bool_oper(lexer)?;
                args.push(arg);
                let comma = lexer.next();
                match comma {
                    Some(Ok(Token::RPar)) => break,
                    Some(Ok(Token::Comma)) => continue,
                    _ => return Err("Expected ',' or ')'".into()),
                }
            }
            SecurityExpression::Call(name, args)
        }
        _ => panic!("Unexpected token {:?}", token),
    };
    Ok(r)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let expr = SecurityExpression::parse("true & false").unwrap();
        assert_eq!(expr.to_string(), "true & false");
    }

    #[test]
    fn test_parse_func() {
        let expr = SecurityExpression::parse("U_SEC() >= 50").unwrap();
        assert_eq!(expr.to_string(), "U_SEC() >= 50");
    }

    #[test]
    fn test_parse_func2() {
        let expr = SecurityExpression::parse("GROUP(\"SYSOPS\")").unwrap();
        assert_eq!(expr.to_string(), "GROUP(\"SYSOPS\")");
    }

    #[test]
    fn test_eval_sec() {
        let expr = SecurityExpression::parse("U_SEC() >= 50").unwrap();
        let mut session = Session::new();
        session.cur_security = 100;
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(true));
        session.cur_security = 10;
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_group_func() {
        let expr = SecurityExpression::parse("U_GROUP(\"FOOBAR\")").unwrap();
        let mut session = Session::new();
        session.cur_groups = vec!["FOOBAR".to_string()];
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(true));
        let expr = SecurityExpression::parse("U_GROUP(\"FOOBAR2\")").unwrap();
        let mut session = Session::new();
        session.cur_groups = vec!["FOOBAR".to_string()];
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(false));
    }
}
