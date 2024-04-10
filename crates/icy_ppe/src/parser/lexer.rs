use crate::{
    ast::{constant::BUILTIN_CONSTS, Constant},
    parser::load_with_encoding,
};
use core::fmt;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use unicase::Ascii;

use super::{Encoding, ErrorRepoter};

#[derive(Error, Default, Debug, Clone, PartialEq)]
pub enum LexingErrorType {
    #[default]
    #[error("Invalid token")]
    InvalidToken,

    #[error("Error parsing number: '{0}' from {1}")]
    InvalidInteger(String, String),

    #[error("Unexpected end of file in string")]
    UnexpectedEOFInString,

    #[error("Error loading include file '{0}': {1}")]
    ErrorLoadingIncludeFile(String, String),

    #[error("Can't find parent of path {0}")]
    PathError(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexingError {
    pub error: LexingErrorType,
    pub range: core::ops::Range<usize>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Spanned<T>
where
    T: PartialEq + Clone,
{
    pub token: T,
    pub span: core::ops::Range<usize>,
}

impl<T: PartialEq + Clone> Spanned<T> {
    pub fn new(token: T, span: core::ops::Range<usize>) -> Self {
        Self { token, span }
    }

    pub fn create_empty(token: T) -> Self {
        Self { token, span: 0..0 }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommentType {
    SingleLineQuote,
    SingleLineSemicolon,
    SingleLineStar,
}

impl fmt::Display for CommentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommentType::SingleLineSemicolon => write!(f, ";"),
            CommentType::SingleLineQuote => write!(f, "'"),
            CommentType::SingleLineStar => write!(f, "*"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Eol,

    Identifier(Ascii<String>),
    Comment(CommentType, String),
    UseFuncs(CommentType, String),

    Comma,

    DotDot,

    LPar,
    RPar,

    PoW,
    Mul,

    Div,
    Mod,
    Add,
    Sub,

    Eq,

    NotEq,
    Lower,

    LowerEq,
    Greater,

    GreaterEq,

    And,
    Or,
    Not,

    If,
    Let,
    While,
    EndWhile,
    Else,
    ElseIf,
    EndIf,
    For,
    Next,
    Break,
    Continue,
    Return,
    Gosub,
    Goto,

    Select,
    Case,
    Default,
    EndSelect,

    Label(unicase::Ascii<String>),

    Declare,
    Function,
    Procedure,
    EndProc,
    EndFunc,

    Const(Constant),
}

impl Token {
    pub fn token_can_be_identifier(&self) -> bool {
        matches!(
            self,
            Token::Identifier(_)
                | Token::If
                | Token::Let
                | Token::While
                | Token::EndWhile
                | Token::Else
                | Token::ElseIf
                | Token::EndIf
                | Token::For
                | Token::Next
                | Token::Break
                | Token::Continue
                | Token::Return
                | Token::Gosub
                | Token::Goto
                | Token::Select
                | Token::Case
                | Token::Default
                | Token::EndSelect
                | Token::Declare
                | Token::Function
                | Token::Procedure
                | Token::EndProc
                | Token::EndFunc
        )
    }

    pub(crate) fn get_identifier(&self) -> Ascii<String> {
        Ascii::new(self.to_string())
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Const(c) => write!(f, "{c}"),
            Token::Identifier(s) => write!(f, "{s}"),
            Token::LPar => write!(f, "("),
            Token::RPar => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::PoW => write!(f, "**"),
            Token::Mul => write!(f, "*"),
            Token::Div => write!(f, "/"),
            Token::Mod => write!(f, "%"),
            Token::Add => write!(f, "+"),
            Token::Sub => write!(f, "-"),
            Token::Eq => write!(f, "="),
            Token::NotEq => write!(f, "!="),
            Token::Lower => write!(f, "<"),
            Token::LowerEq => write!(f, "<="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEq => write!(f, ">="),
            Token::And => write!(f, "&"),
            Token::Or => write!(f, "|"),
            Token::Not => write!(f, "!"),
            Token::DotDot => write!(f, ".."),

            Token::Label(s) => write!(f, ":{s}"),

            Token::Let => write!(f, "LET"),
            Token::While => write!(f, "WHILE"),
            Token::EndWhile => write!(f, "ENDWHILE"),
            Token::If => write!(f, "IF"),
            Token::Else => write!(f, "ELSE"),
            Token::ElseIf => write!(f, "ELSEIF"),
            Token::EndIf => write!(f, "ENDIF"),

            Token::For => write!(f, "FOR"),
            Token::Next => write!(f, "NEXT"),
            Token::Break => write!(f, "BREAK"),
            Token::Continue => write!(f, "CONTINUE"),
            Token::Return => write!(f, "RETURN"),
            Token::Gosub => write!(f, "GOSUB"),
            Token::Goto => write!(f, "GOTO"),

            Token::Select => write!(f, "SELECT"),
            Token::Case => write!(f, "CASE"),
            Token::Default => write!(f, "DEFAULT"),
            Token::EndSelect => write!(f, "ENDSELECT"),

            Token::Comment(ct, s) | Token::UseFuncs(ct, s) => write!(f, "{ct}{s}"),
            Token::Eol => write!(f, "<End Of Line>"),

            // Token::VarType(t) => write!(f, "{:?}", t),
            Token::Declare => write!(f, "DECLARE"),
            Token::Function => write!(f, "FUNCTION"),
            Token::Procedure => write!(f, "PROCEDURE"),
            Token::EndProc => write!(f, "ENDPROC"),
            Token::EndFunc => write!(f, "ENDFUNC"),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LexerState {
    AfterEol,
    AfterColonEol,
    BeyondEOL,
}

pub struct Lexer {
    file: PathBuf,
    encoding: Encoding,
    text: Vec<char>,

    errors: Arc<Mutex<ErrorRepoter>>,
    lexer_state: LexerState,
    token_start: usize,
    token_end: usize,

    include_lexer: Option<Box<Lexer>>,
}

lazy_static::lazy_static! {
    static ref TOKEN_LOOKUP_TABLE: HashMap<unicase::Ascii<String>, Token> = {
        let mut m = HashMap::new();
        m.insert(unicase::Ascii::new("if".to_string()), Token::If);
        m.insert(unicase::Ascii::new("let".to_string()), Token::Let);
        m.insert(unicase::Ascii::new("while".to_string()), Token::While);
        m.insert(unicase::Ascii::new("endwhile".to_string()), Token::EndWhile);
        m.insert(unicase::Ascii::new("else".to_string()), Token::Else);
        m.insert(unicase::Ascii::new("elseif".to_string()), Token::ElseIf);
        m.insert(unicase::Ascii::new("endif".to_string()), Token::EndIf);
        m.insert(unicase::Ascii::new("for".to_string()), Token::For);
        m.insert(unicase::Ascii::new("next".to_string()), Token::Next);
        m.insert(unicase::Ascii::new("endfor".to_string()), Token::Next);

        m.insert(unicase::Ascii::new("break".to_string()), Token::Break);
        m.insert(unicase::Ascii::new("continue".to_string()), Token::Continue);
        m.insert(unicase::Ascii::new("return".to_string()), Token::Return);

        m.insert(unicase::Ascii::new("gosub".to_string()), Token::Gosub);
        m.insert(unicase::Ascii::new("goto".to_string()), Token::Goto);
        m.insert(unicase::Ascii::new("select".to_string()), Token::Select);
        m.insert(unicase::Ascii::new("case".to_string()), Token::Case);
        m.insert(unicase::Ascii::new("default".to_string()), Token::Default);
        m.insert(unicase::Ascii::new("endselect".to_string()), Token::EndSelect);
        m.insert(unicase::Ascii::new("declare".to_string()), Token::Declare);
        m.insert(unicase::Ascii::new("function".to_string()), Token::Function);
        m.insert(unicase::Ascii::new("procedure".to_string()), Token::Procedure);
        m.insert(unicase::Ascii::new("endproc".to_string()), Token::EndProc);
        m.insert(unicase::Ascii::new("endfunc".to_string()), Token::EndFunc);
        for c in &BUILTIN_CONSTS {
            m.insert(unicase::Ascii::new(c.name.to_string()), Token::Const(Constant::Builtin(c)));
        }
        m
    };
}

impl Lexer {
    pub fn new(
        file: PathBuf,
        text: &str,
        encoding: Encoding,
        errors: Arc<Mutex<ErrorRepoter>>,
    ) -> Self {
        Self {
            file,
            encoding,
            text: text.chars().collect(),
            lexer_state: LexerState::AfterEol,
            errors,
            token_start: 0,
            token_end: 0,
            include_lexer: None,
        }
    }

    #[inline]
    fn next_ch(&mut self) -> Option<char> {
        if self.token_end >= self.text.len() {
            None
        } else {
            let t = self.text[self.token_end];
            // Some files take that as end of file char.
            if t == '\x1A' {
                return None;
            }
            self.token_end += 1;
            Some(t)
        }
    }

    #[inline]
    fn put_back(&mut self) {
        self.token_end -= 1;
    }

    /// Returns the next token of this [`Lexer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn next_token(&mut self) -> Option<Token> {
        if let Some(lexer) = &mut self.include_lexer {
            let result = lexer.next_token();
            match result {
                Some(token) => {
                    return Some(token);
                }
                None => {
                    self.include_lexer = None;
                }
            }
        }
        let ch;
        loop {
            self.token_start = self.token_end;
            if let Some(next_ch) = self.next_ch() {
                if next_ch != ' ' && next_ch != '\t' {
                    ch = next_ch;
                    break;
                }
            } else {
                return None;
            }
        }
        let state = match ch {
            '\'' | // comment
            ';' => {
                return self.read_comment(ch);
            }
            '"' => {
                let mut string_result = String::new();
                loop {
                    let Some(sch) = self.next_ch() else {
                        self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::UnexpectedEOFInString);
                        return None;
                    };
                    if sch == '"'  {
                        if let Some('"') = self.next_ch() {
                            string_result.push('"');
                            continue;
                        };
                        self.put_back();
                        break;
                    }
                    string_result.push(sch);
                }
                Some(Token::Const(Constant::String(string_result)))
            }
            '\\' => { // eol continuation
                let next = self.next_ch();
                if let Some('\r') = next {
                    if let Some('\n') = self.next_ch() {
                        return self.next_token();
                    }
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;
                }
                if let Some('\n') = next {
                    return self.next_token();
                }
                self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                return None;

            },
            '_' => { // eol continuation
                let next = self.next_ch();
                if let Some('\r') = next {
                    if let Some('\n') = self.next_ch() {
                        return self.next_token();
                    }
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;

                }
                if let Some('\n') = next {
                    return self.next_token();
                }
                return self.read_identifier();
            },
            '\r' => {
                return if let Some('\n') = self.next_ch() {
                    self.lexer_state = LexerState::AfterEol;
                    Some(Token::Eol)
                } else {
                    self.put_back();
                    self.lexer_state = LexerState::AfterEol;
                    Some(Token::Eol)
                };
            },
            '\n' => {
                self.lexer_state = LexerState::AfterEol;
                return Some(Token::Eol);
            },
            ':' => {
                if self.lexer_state == LexerState::BeyondEOL {
                    self.lexer_state = LexerState::AfterColonEol;
                    return Some(Token::Eol);
                }
                let mut got_non_ws = false;
                let mut label_start = 0;
                loop {
                    let Some(ch) = self.next_ch() else {
                        break;
                    };
                    if !got_non_ws && (ch == ' ' || ch == '\t') {
                        label_start += 1;
                        continue;
                    }
                    //assert!(ch.is_some(), "Unexpected eof in string_literal at ({}, {}).", self.line, self.col);
                    if !(ch.is_ascii_alphanumeric() || "_@#$¢£¥€".contains(ch)) {
                        self.put_back();
                        break;
                    }
                    got_non_ws = true;
                }

                let identifier = unicase::Ascii::new(self.text[self.token_start+1+label_start..self.token_end].iter().collect::<String>());
                Some(Token::Label(identifier))
            },

            '(' | '[' | '{' => Some(Token::LPar),
            ')' | ']' | '}' => Some(Token::RPar),
            ',' => Some(Token::Comma),
            '^' => Some(Token::PoW),
            '*' => {
                if self.lexer_state != LexerState::BeyondEOL {
                    return self.read_comment(ch);
                }
                let next = self.next_ch();
                 if let Some('*') = next {
                    Some(Token::PoW)
                } else {
                    self.put_back();
                    Some(Token::Mul)
                }
             },
            '/' => Some(Token::Div),
            '%' => Some(Token::Mod),
            '+' => Some(Token::Add),
            '-' => Some(Token::Sub),
            '=' => {
                let next = self.next_ch();
                 match next {
                    Some('<') => Some(Token::LowerEq),
                    Some('>') => Some(Token::GreaterEq),
                    Some('=') => Some(Token::Eq),
                     _ => {
                         self.put_back();
                         Some(Token::Eq)
                     }
                 }
             },
            '&'  => {
                let next = self.next_ch();
                 if let Some('&') = next {
                    Some(Token::And)
                } else {
                                         self.put_back();
                                         Some(Token::And)
                                     }
             },
            '|' => {
                let next = self.next_ch();
                 if let Some('|') = next {
                    Some(Token::Or)
                } else {
                    self.put_back();
                    Some(Token::Or)
                }
             },
            '!' => {
                let next = self.next_ch();
                 if let Some('=') = next {
                    Some(Token::NotEq)
                } else {
                    self.put_back();
                    Some(Token::Not)
                }
             },
            '@' => {
                let ch = self.next_ch();
                if Some('X') != ch && Some('x') != ch {
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;

                }
                let Some(first) = self.next_ch() else {
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;

                };
                let Some(second) = self.next_ch() else {
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;

                };
                if !first.is_ascii_hexdigit() || !second.is_ascii_hexdigit() {
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;

                }
                Some(Token::Const(Constant::Integer(conv_hex(first) * 16 + conv_hex(second))))
            }
            '$' => {
                let mut identifier = String::new();
                let mut is_last = false;
                loop {
                    let Some(ch) = self.next_ch() else {
                        is_last = true;
                        break;
                    };
                    if !ch.is_ascii_digit() && ch != '.' {
                        break;
                    }
                    identifier.push(ch);
                }
                if !is_last {
                    self.put_back();
                }
                let Ok(r) = identifier.parse::<f64>() else {
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;
                };
                Some(Token::Const(Constant::Money((r * 100.0) as i32)))
            }

            '<' => {
                let next = self.next_ch();
                match next {
                    Some('>') => Some(Token::NotEq),
                    Some('=') => Some(Token::LowerEq),
                    _ => {
                        self.put_back();
                        Some(Token::Lower)
                    }
                }
            },
            '>' => {
                let next = self.next_ch();
                match next {
                     Some('<') => Some(Token::NotEq),
                     Some('=') => Some(Token::GreaterEq),
                     _ => {
                         self.put_back();
                         Some(Token::Greater)
                     }
                 }
             }
             '.' => {
                let next = self.next_ch();
                if next == Some('.') {
                    Some(Token::DotDot)
                } else {
                    self.put_back();
                    self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                    return None;
                }
             }
            _ => {
                if ch.is_ascii_alphabetic() || ch == '_' {
                    return self.read_identifier();
                }

                if ch.is_ascii_digit() {
                    self.lexer_state = LexerState::BeyondEOL;

                    let start = self.token_start;
                    let mut cur_ch = ch;
                    loop {
                        let Some(ch) = self.next_ch() else {
                            break;
                        };
                        cur_ch = ch;

                        match ch {
                            '.' => {  break; }
                            'D' | 'd' => {
                                let r = self.text[start..self.token_end - 1].iter().collect::<String>().parse::<i32>();
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );
                                        return Some(Token::Const(Constant::Integer(-1)));
                                    }
                                }
                            }
                            'H' | 'h' => {
                                let r = i32::from_str_radix(&self.text[start..self.token_end - 1].iter().collect::<String>(), 16);
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );
                                        return Some(Token::Const(Constant::Integer(-1)));
                                    }
                                }
                            }
                            'O' | 'o' => {
                                let r = i32::from_str_radix(&self.text[start..self.token_end - 1].iter().collect::<String>(), 8);
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );
                                        return Some(Token::Const(Constant::Integer(-1)));
                                    }
                                }
                            }
                            'B' | 'b' => {
                                if let Some(ch) = self.next_ch()  {
                                    if ch.is_ascii_hexdigit() {
                                        continue;
                                    }
                                    self.put_back();
                                }

                                let r = i32::from_str_radix(&self.text[start..self.token_end - 1].iter().collect::<String>(), 2);

                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );

                                        return Some(Token::Const(Constant::Integer(-1)));
                                    }
                                }
                            }
                            _ => {}
                        }
                        if !ch.is_ascii_hexdigit()  {
                            self.put_back();
                            break;
                        }
                    }
                    let mut end = self.token_end;
                    if cur_ch == '.' {
                        let mut found_dot_dot = false;
                        if let Some(ch) = self.next_ch()  {
                            // got dotdot, put back
                            if ch == '.' {
                                self.put_back();
                                self.put_back();
                                end -= 1;
                                found_dot_dot = true;
                            }
                        } else {
                            self.put_back();
                        }
                        if !found_dot_dot {
                            let mut is_last = false;
                            loop {
                                let Some(ch) = self.next_ch() else {
                                    is_last = true;
                                    break;
                                };
                                if !ch.is_ascii_digit() && ch != '.' {
                                    break;
                                }
                            }
                            if !is_last {
                                self.put_back();
                            }
                            end = self.token_end;
                            let r = self.text[start..end].iter().collect::<String>().parse::<f64>();
                            match r {
                                Ok(f) => {
                                    return Some(Token::Const(Constant::Double(f)));
                                }
                                Err(r) => {
                                    self.errors.lock().unwrap().report_warning(
                                        self.token_start..self.token_end,
                                        LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                    );
                                    return Some(Token::Const(Constant::Double(-1.0)));
                                }
                            }
                        }
                    }

                    let r = self.text[start..end].iter().collect::<String>().parse::<i64>();
                    match r {
                        Ok(i) => {
                            if i32::try_from(i).is_ok()  {
                                return Some(Token::Const(Constant::Integer(i as i32)));
                            }
                            if i >= 0 {
                                return Some(Token::Const(Constant::Unsigned(i as u64)));
                            }
                        }
                        Err(r) => {
                            let r2 = self.text[start..end].iter().collect::<String>().parse::<u64>();
                            if let Ok(i) = r2 {
                                return Some(Token::Const(Constant::Unsigned(i)));
                            }
                            self.errors.lock().unwrap().report_warning(
                                self.token_start..self.token_end,
                                LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                            );
                            return Some(Token::Const(Constant::Integer(-1)));
                        }
                    }
                }

                self.errors.lock().unwrap().report_error(self.token_start..self.token_end,LexingErrorType::InvalidToken);
                return None;

            }
        };
        self.lexer_state = LexerState::BeyondEOL;
        state
    }

    #[allow(clippy::unnecessary_wraps)]
    fn read_identifier(&mut self) -> Option<Token> {
        self.lexer_state = LexerState::BeyondEOL;
        let mut open_bracket = false;
        loop {
            let Some(ch) = self.next_ch() else {
                break;
            };
            //assert!(ch.is_some(), "Unexpected eof in string_literal at ({}, {}).", self.line, self.col);
            if !(ch.is_ascii_alphanumeric() || "_@#$¢£¥€".contains(ch)) {
                let mut ch2 = ch;
                while ch2 == ' ' && ch2 == '\t' {
                    let Some(ch) = self.next_ch() else {
                        break;
                    };
                    ch2 = ch;
                }
                if ch2 == '(' || ch2 == '[' || ch2 == '{' {
                    open_bracket = true;
                }
                self.put_back();
                break;
            }
        }

        let identifier = unicase::Ascii::new(
            self.text[self.token_start..self.token_end]
                .iter()
                .collect::<String>(),
        );
        if !open_bracket {
            if let Some(token) = TOKEN_LOOKUP_TABLE.get(&identifier) {
                return Some(token.clone());
            }
        }
        Some(Token::Identifier(identifier))
    }

    fn read_comment(&mut self, ch: char) -> Option<Token> {
        let cmt_type = match ch {
            ';' => CommentType::SingleLineSemicolon,
            '*' => CommentType::SingleLineStar,
            _ => CommentType::SingleLineQuote,
        };
        loop {
            let Some(ch) = self.next_ch() else {
                break;
            };
            if ch == '\n' {
                break;
            }
        }
        self.lexer_state = LexerState::AfterEol;
        let comment = self.text[self.token_start + 1..self.token_end]
            .iter()
            .collect::<String>();

        if comment.len() > "$INCLUDE:".len()
            && comment
                .chars()
                .take("$INCLUDE:".len())
                .collect::<String>()
                .to_ascii_uppercase()
                == "$INCLUDE:"
        {
            let include_file = comment
                .chars()
                .skip("$INCLUDE:".len())
                .collect::<String>()
                .trim()
                .to_string();
            let Some(parent) = self.file.parent() else {
                self.errors.lock().unwrap().report_error(
                    self.token_start..self.token_end,
                    LexingErrorType::PathError(self.file.to_string_lossy().to_string()),
                );
                return None;
            };
            let path = parent.join(include_file.clone());

            match load_with_encoding(&path, self.encoding) {
                Ok(k) => {
                    self.include_lexer = Some(Box::new(Lexer::new(
                        path,
                        &k,
                        Encoding::Utf8,
                        self.errors.clone(),
                    )));
                }
                Err(err) => {
                    self.errors.lock().unwrap().report_error(
                        self.token_start..self.token_end,
                        LexingErrorType::ErrorLoadingIncludeFile(
                            include_file.to_string(),
                            err.to_string(),
                        ),
                    );
                    return None;
                }
            }
        }

        if comment.len() > "$USEFUNCS".len()
            && comment
                .chars()
                .take("$USEFUNCS".len())
                .collect::<String>()
                .to_ascii_uppercase()
                == "$USEFUNCS"
        {
            return Some(Token::UseFuncs(cmt_type, comment));
        }
        Some(Token::Comment(cmt_type, comment))
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.token_start..self.token_end
    }
}

fn conv_hex(first: char) -> i32 {
    if first.is_ascii_digit() {
        return first as i32 - b'0' as i32;
    }
    if ('a'..='f').contains(&first) {
        return first as i32 - b'a' as i32 + 10;
    }
    if ('A'..='F').contains(&first) {
        return first as i32 - b'A' as i32 + 10;
    }
    0
}
