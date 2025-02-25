use std::{fmt::Display, iter::Peekable, str::FromStr};

use chrono::{Local, NaiveTime, Timelike};
use logos::{Lexer, Logos};
use serde::{Deserialize, Serialize};

use super::state::Session;
use crate::{Res, datetime::IcbDate};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(PartialEq, Clone, Eq, Debug, Serialize, Deserialize)]
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

#[derive(Clone, PartialEq)]
pub enum SecurityExpression {
    UnaryExpression(UnaryOp, Box<SecurityExpression>),
    BinaryExpression(BinaryOp, Box<SecurityExpression>, Box<SecurityExpression>),
    Call(String, Vec<SecurityExpression>),
    Constant(Value),
    Parens(Box<SecurityExpression>),
}

impl Default for SecurityExpression {
    fn default() -> Self {
        SecurityExpression::Constant(Value::Integer(0))
    }
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

impl FromStr for SecurityExpression {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(SecurityExpression::default());
        }
        let mut a = Token::lexer(s).peekable();
        Ok(bool_oper(&mut a)?)
    }
}

impl From<String> for SecurityExpression {
    fn from(s: String) -> Self {
        SecurityExpression::from_str(&s).unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for SecurityExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}

impl serde::Serialize for SecurityExpression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
enum Token {
    // Tokens can be literal strings, of any length.
    #[token("true", |_| true, ignore(ascii_case))]
    #[token("false", |_| false, ignore(ascii_case))]
    Bool(bool),

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
                U_SEC_FUNC => Ok(Value::Integer(session.cur_security as i64)),

                U_AGE_FUNC => {
                    let age = if let Some(user) = &session.current_user {
                        chrono::Utc::now().years_since(user.birth_date.to_utc_date_time()).unwrap_or(0)
                    } else {
                        0
                    };
                    Ok(Value::Integer(age as i64))
                }

                U_GROUP_FUNC => {
                    if let Value::String(group) = args[0].eval(session)? {
                        Ok(Value::Bool(session.cur_groups.contains(&group)))
                    } else {
                        Err("Invalid argument for U_GROUP".into())
                    }
                }

                TIME_FUNC => {
                    let time = Local::now().time();
                    Ok(Value::Time(time))
                }

                TIME_LEFT_FUNC => Ok(Value::Integer(session.minutes_left() as i64)),

                DOW_FUNC => Ok(Value::Integer(IcbDate::today().day_of_week() as i64)),
                _ => Err(format!("Invalid function {name}").into()),
            },
            SecurityExpression::Constant(constant) => Ok(constant.clone()),
            SecurityExpression::Parens(expr) => expr.eval(session),
        }
    }

    pub fn user_can_access(&self, session: &Session) -> bool {
        match self.eval(session) {
            Ok(Value::Bool(b)) => b,
            Ok(Value::Integer(i)) => session.cur_security >= i as u8,
            Ok(_) => {
                log::error!("expression didn't evaluate to bool ({})", self);
                false
            }
            Err(err) => {
                log::error!("Error evaluating security expression: {} ({})", err, self);
                false
            }
        }
    }

    pub fn from_req_security(security: u8) -> SecurityExpression {
        SecurityExpression::Constant(Value::Integer(security as i64))
    }

    pub fn is_empty(&self) -> bool {
        match self {
            SecurityExpression::Constant(Value::Bool(true)) => true,
            _ => false,
        }
    }

    pub(crate) fn level(&self) -> u8 {
        // TODO
        0
    }

    /// TODO: Implement me.
    pub fn to_pcb_sec(&self) -> i32 {
        10
    }
}

pub const U_SEC_FUNC: &str = "U_SEC";
pub const U_AGE_FUNC: &str = "U_AGE";
pub const U_GROUP_FUNC: &str = "U_GROUP";
pub const TIME_FUNC: &str = "TIME";
pub const TIME_LEFT_FUNC: &str = "TIME_LEFT";
pub const DOW_FUNC: &str = "DOW";

fn bool_oper(lexer: &mut Peekable<Lexer<Token>>) -> Result<SecurityExpression, String> {
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

fn eq_oper(lexer: &mut Peekable<Lexer<Token>>) -> Result<SecurityExpression, String> {
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

fn factor(lexer: &mut Peekable<Lexer<Token>>) -> Result<SecurityExpression, String> {
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
        Token::Bool(b) => SecurityExpression::Constant(Value::Bool(b)),
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
        let expr = SecurityExpression::from_str("true & false").unwrap();
        assert_eq!(expr.to_string(), "true & false");
    }

    #[test]
    fn test_parse_func() {
        let expr = SecurityExpression::from_str("U_SEC() >= 50").unwrap();
        assert_eq!(expr.to_string(), "U_SEC() >= 50");
    }

    #[test]
    fn test_parse_func2() {
        let expr = SecurityExpression::from_str("GROUP(\"SYSOPS\")").unwrap();
        assert_eq!(expr.to_string(), "GROUP(\"SYSOPS\")");
    }

    #[test]
    fn test_eval_sec() {
        let expr = SecurityExpression::from_str("U_SEC() >= 50").unwrap();
        let mut session = Session::new();
        session.cur_security = 100;
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(true));
        session.cur_security = 10;
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_eval_time() {
        let expr = SecurityExpression::from_str("10:00 < 12:00").unwrap();
        let session = Session::new();
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_group_func() {
        let expr = SecurityExpression::from_str("U_GROUP(\"FOOBAR\")").unwrap();
        let mut session = Session::new();
        session.cur_groups = vec!["FOOBAR".to_string()];
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(true));
        let expr = SecurityExpression::from_str("U_GROUP(\"FOOBAR2\")").unwrap();
        let mut session = Session::new();
        session.cur_groups = vec!["FOOBAR".to_string()];
        assert_eq!(expr.eval(&session).unwrap(), Value::Bool(false));
    }
}
