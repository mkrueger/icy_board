use crate::{
    ast::{
        constant::{NumberFormat, BUILTIN_CONSTS},
        Constant, Statement,
    },
    compiler::workspace::Workspace,
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

use super::{pre_processor_expr_visitor::PreProcessorVisitor, Encoding, ErrorReporter, Parser, UserTypeRegistry};

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

    #[error("Use ^ instead of **")]
    PowWillGetRemoved,

    #[error("Don't use braces, they will get another meaning in the future. Use '(', ')' instead.")]
    DontUseBraces,

    #[error("Invalid define value: {0}")]
    InvalidDefineValue(String),

    #[error("Already defined ({0})")]
    AlreadyDefined(String),

    #[error("$ELSE without $IF")]
    ElseWithoutIf,

    #[error("$ELIF without $IF")]
    ElseIfWithoutIf,

    #[error("Missing $ENDIF")]
    MissingEndIf,
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
    BlockComment,
}

impl fmt::Display for CommentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommentType::SingleLineSemicolon => write!(f, ";"),
            CommentType::SingleLineQuote => write!(f, "'"),
            CommentType::SingleLineStar => write!(f, "*"),
            CommentType::BlockComment => write!(f, ""),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Eol,

    Identifier(Ascii<String>),
    Comment(CommentType, String),
    UseFuncs(CommentType, String),
    Define(CommentType, String, Constant),

    Comma,

    Dot,
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

    // New in 400
    Repeat,
    Until,

    LBrace,
    RBrace,

    LBracket,
    RBracket,

    Loop,
    EndLoop,

    MulAssign,

    DivAssign,
    ModAssign,
    AddAssign,
    SubAssign,
    AndAssign,
    OrAssign,
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
                | Token::Repeat
                | Token::Until
                | Token::Loop
                | Token::EndLoop
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
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::PoW => write!(f, "^"),
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
            Token::Dot => write!(f, "."),
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
            Token::Define(ct, s, value) => write!(f, "{ct}DEFINE {s} = {value}"),

            Token::Eol => write!(f, "<End Of Line>"),

            // Token::VarType(t) => write!(f, "{:?}", t),
            Token::Declare => write!(f, "DECLARE"),
            Token::Function => write!(f, "FUNCTION"),
            Token::Procedure => write!(f, "PROCEDURE"),
            Token::EndProc => write!(f, "ENDPROC"),
            Token::EndFunc => write!(f, "ENDFUNC"),

            Token::Repeat => write!(f, "REPEAT"),
            Token::Until => write!(f, "UNTIL"),
            Token::Loop => write!(f, "LOOP"),
            Token::EndLoop => write!(f, "ENDLOOP"),
            Token::MulAssign => write!(f, "*="),
            Token::DivAssign => write!(f, "/="),
            Token::ModAssign => write!(f, "%="),
            Token::AddAssign => write!(f, "+="),
            Token::SubAssign => write!(f, "-="),
            Token::AndAssign => write!(f, "&="),
            Token::OrAssign => write!(f, "|="),
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
    lookup_table: &'static HashMap<unicase::Ascii<String>, Token>,
    define_table: HashMap<unicase::Ascii<String>, Constant>,
    file: PathBuf,
    lang_version: u16,
    encoding: Encoding,
    text: Vec<char>,

    errors: Arc<Mutex<ErrorReporter>>,
    lexer_state: LexerState,
    token_start: usize,
    token_end: usize,
    if_level: i32,

    include_lexer: Option<Box<Lexer>>,
}

lazy_static::lazy_static! {
    static ref TOKEN_LOOKUP_TABLE_100: HashMap<unicase::Ascii<String>, Token> = {
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

        for c in &BUILTIN_CONSTS {
            m.insert(unicase::Ascii::new(c.name.to_string()), Token::Const(Constant::Builtin(c)));
        }
        m
    };

    static ref TOKEN_LOOKUP_TABLE_200: HashMap<unicase::Ascii<String>, Token> = {
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

        for c in &BUILTIN_CONSTS {
            m.insert(unicase::Ascii::new(c.name.to_string()), Token::Const(Constant::Builtin(c)));
        }
        m
    };
    static ref TOKEN_LOOKUP_TABLE_300: HashMap<unicase::Ascii<String>, Token> = {
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

    static ref TOKEN_LOOKUP_TABLE_350: HashMap<unicase::Ascii<String>, Token> = {
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

        // new ones
        m.insert(unicase::Ascii::new("repeat".to_string()), Token::Repeat);
        m.insert(unicase::Ascii::new("until".to_string()), Token::Until);
        m.insert(unicase::Ascii::new("loop".to_string()), Token::Loop);
        m.insert(unicase::Ascii::new("endloop".to_string()), Token::EndLoop);

        for c in &BUILTIN_CONSTS {
            m.insert(unicase::Ascii::new(c.name.to_string()), Token::Const(Constant::Builtin(c)));
        }
        m
    };

}

impl Lexer {
    pub fn new(file: PathBuf, workspace: &Workspace, text: &str, encoding: Encoding, errors: Arc<Mutex<ErrorReporter>>) -> Self {
        let mut define_table = HashMap::new();
        define_table.insert(Ascii::new("VERSION".into()), Constant::String(workspace.package.version.to_string()));
        let lang_version = workspace.language_version();
        define_table.insert(Ascii::new("LANGVERSION".into()), Constant::Integer(lang_version as i32, NumberFormat::Default));
        define_table.insert(
            Ascii::new("RUNTIME".into()),
            Constant::Integer(workspace.runtime() as i32, NumberFormat::Default),
        );

        Self {
            lookup_table: if lang_version < 200 {
                &*TOKEN_LOOKUP_TABLE_100
            } else if lang_version < 300 {
                &*TOKEN_LOOKUP_TABLE_200
            } else if lang_version < 350 {
                &*TOKEN_LOOKUP_TABLE_300
            } else {
                &*TOKEN_LOOKUP_TABLE_350
            },
            file,
            lang_version,
            define_table,
            encoding,
            text: text.chars().collect(),
            lexer_state: LexerState::AfterEol,
            errors,
            token_start: 0,
            token_end: 0,
            include_lexer: None,
            if_level: 0,
        }
    }

    pub fn get_define(&self, key: &str) -> Option<&Constant> {
        self.define_table.get(&Ascii::new(key.to_string()))
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
                    self.check_eof();
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
                self.check_eof();
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
                        match self.next_ch() {
                            Some('"') => {
                                string_result.push('"');
                                continue;
                            }
                            None => {
                                break;
                            }
                            _ => {
                                self.put_back();
                                break;
                            }
                        }
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

            '(' => Some(Token::LPar),
            ')' => Some(Token::RPar),

            '['  => {
                if self.lang_version < 350 {
                    Some(Token::LPar)
                } else {
                    Some(Token::LBracket)
                }
            }

            ']'  => {
                if self.lang_version < 350 {
                    Some(Token::RPar)
                } else {
                    Some(Token::RBracket)
                }
            }

            '{'  => {
                if self.lang_version < 350 {
                    self.errors.lock().unwrap().report_warning(self.token_start..self.token_end,LexingErrorType::DontUseBraces);
                    Some(Token::LPar)
                } else {
                    Some(Token::LBrace)
                }
            }

            '}'  => {
                if self.lang_version < 350 {
                    self.errors.lock().unwrap().report_warning(self.token_start..self.token_end,LexingErrorType::DontUseBraces);
                    Some(Token::RPar)
                } else {
                    Some(Token::RBrace)
                }
            }

            ',' => Some(Token::Comma),
            '^' => Some(Token::PoW),
            '*' => {
                if self.lexer_state != LexerState::BeyondEOL {
                    return self.read_comment(ch);
                }
                let next = self.next_ch();
                if let Some('*') = next {
                    self.errors.lock().unwrap().report_warning(self.token_start..self.token_end,LexingErrorType::PowWillGetRemoved);
                    Some(Token::PoW)
                } else {
                    if self.lang_version >= 350 && next == Some('=') {
                        return Some(Token::MulAssign);
                    }
                    self.put_back();
                    Some(Token::Mul)
                }
             },
            '/' => {
                if self.lang_version >= 350 {
                    let next = self.next_ch();
                    if next == Some('=') {
                        return Some(Token::DivAssign);
                    }
                    self.put_back();
                }
                Some(Token::Div)
            },
            '%' => {
                if self.lang_version >= 350 {
                    let next = self.next_ch();
                    if next == Some('=') {
                        return Some(Token::ModAssign);
                    }
                    self.put_back();
                }
                Some(Token::Mod)
            }
            '+' => {
                if self.lang_version >= 350 {
                    let next = self.next_ch();
                    if next == Some('=') {
                        return Some(Token::AddAssign);
                    }
                    self.put_back();
                }
                Some(Token::Add)
            }
            '-' => {
                if self.lang_version >= 350 {
                    let next = self.next_ch();
                    if next == Some('=') {
                        return Some(Token::SubAssign);
                    }
                    self.put_back();
                }
                Some(Token::Sub)
            }
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
                    if self.lang_version >= 350 {
                        if next == Some('=') {
                            return Some(Token::AndAssign);
                        }
                    }

                    self.put_back();
                    Some(Token::And)
                }
             },
            '|' => {
                let next = self.next_ch();
                 if let Some('|') = next {
                    Some(Token::Or)
                } else {
                    if self.lang_version >= 350 {
                        if next == Some('=') {
                            return Some(Token::OrAssign);
                        }
                    }
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
                Some(Token::Const(Constant::Integer(conv_hex(first) * 16 + conv_hex(second), NumberFormat::ColorCode)))
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

                    if self.lang_version >= 400 {
                        return Some(Token::Dot);
                    }

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
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Dec)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );
                                        return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
                                    }
                                }
                            }
                            'H' | 'h' => {
                                let r = i32::from_str_radix(&self.text[start..self.token_end - 1].iter().collect::<String>(), 16);
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Hex)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );
                                        return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
                                    }
                                }
                            }
                            'O' | 'o' => {
                                let r = i32::from_str_radix(&self.text[start..self.token_end - 1].iter().collect::<String>(), 8);
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Octal)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );
                                        return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
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
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Binary)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>())
                                        );

                                        return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
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
                                return Some(Token::Const(Constant::Integer(i as i32, NumberFormat::Default)));
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
                            return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
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

        let identifier = unicase::Ascii::new(self.text[self.token_start..self.token_end].iter().collect::<String>());
        if !open_bracket {
            if let Some(token) = self.lookup_table.get(&identifier) {
                return Some(token.clone());
            }
        }
        Some(Token::Identifier(identifier))
    }

    fn read_define(&mut self) -> Option<Token> {
        let mut define = String::new();
        loop {
            let Some(ch) = self.next_ch() else {
                break;
            };
            if !char::is_alphanumeric(ch) {
                break;
            }
            define.push(ch);
        }

        if let Some(value) = self.define_table.get(&Ascii::new(define)) {
            Some(Token::Const(value.clone()))
        } else {
            None
        }
    }

    fn read_comment(&mut self, ch: char) -> Option<Token> {
        let cmt_type = match ch {
            ';' => CommentType::SingleLineSemicolon,
            '*' => CommentType::SingleLineStar,
            _ => CommentType::SingleLineQuote,
        };
        let mut comment = Vec::new();
        loop {
            let Some(ch) = self.next_ch() else {
                break;
            };
            if ch == '\n' {
                break;
            }
            if comment.is_empty() && ch == '#' {
                return self.read_define();
            }
            comment.push(ch);
        }
        self.lexer_state = LexerState::AfterEol;

        if comment.len() > "$INCLUDE:".len() && comment.iter().take("$INCLUDE:".len()).collect::<String>().to_ascii_uppercase() == "$INCLUDE:" {
            let include_file = comment.iter().skip("$INCLUDE:".len()).collect::<String>().trim().to_string();
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
                    self.include_lexer = Some(Box::new(Self {
                        lookup_table: self.lookup_table,
                        file: path.clone(),
                        lang_version: self.lang_version,
                        define_table: self.define_table.clone(),
                        encoding: self.encoding,
                        text: k.chars().collect(),
                        lexer_state: LexerState::AfterEol,
                        errors: self.errors.clone(),
                        token_start: 0,
                        token_end: 0,
                        include_lexer: None,
                        if_level: 0,
                    }));
                }
                Err(err) => {
                    self.errors.lock().unwrap().report_error(
                        self.token_start..self.token_end,
                        LexingErrorType::ErrorLoadingIncludeFile(include_file.to_string(), err.to_string()),
                    );
                    return None;
                }
            }
        }
        if comment.len() > "$USEFUNCS".len() && comment.iter().take("$USEFUNCS".len()).collect::<String>().to_ascii_uppercase() == "$USEFUNCS" {
            return Some(Token::UseFuncs(cmt_type, comment.iter().collect()));
        }
        if comment.len() >= "$ENDIF".len() && comment.iter().take("$ENDIF".len()).collect::<String>().to_ascii_uppercase() == "$ENDIF" {
            self.if_level -= 1;
        }

        if comment.len() > "$ELSE".len() && comment.iter().take("$ELSE".len()).collect::<String>().to_ascii_uppercase() == "$ELSE" {
            if self.if_level == 0 {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::ElseWithoutIf);
            }
            loop {
                let Some(ch) = self.next_ch() else {
                    break;
                };
                comment.push(ch);
                if comment.ends_with(&";$ENDIF".chars().collect::<Vec<char>>()) {
                    self.if_level -= 1;
                    break;
                }
            }
            return Some(Token::Comment(cmt_type, comment.iter().collect()));
        }

        let is_if = comment.len() > "$IF".len() && comment.iter().take("$IF".len()).collect::<String>().to_ascii_uppercase() == "$IF";
        let if_elif = comment.len() > "$ELIF".len() && comment.iter().take("$ELIF".len()).collect::<String>().to_ascii_uppercase() == "$ELIF";
        if is_if || if_elif {
            if is_if {
                self.if_level += 1;
            } else {
                if self.if_level == 0 {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::ElseIfWithoutIf);
                }
            }
            let reg = UserTypeRegistry::default();
            let input = comment.iter().skip(if is_if { "$IF".len() } else { "$ELIF".len() }).collect::<String>();
            let mut parser = Parser::new(PathBuf::from("."), self.errors.clone(), &reg, &input, Encoding::Utf8, &Workspace::default());
            parser.next_token();
            let res = parser.parse_expression().unwrap();
            let mut visitor = PreProcessorVisitor {
                define_table: &self.define_table,
                errors: self.errors.clone(),
            };
            let value = res.visit(&mut visitor);
            if let Some(value) = value {
                if value.as_bool() {
                    return Some(Token::Comment(cmt_type, comment.iter().collect()));
                }
            } else {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                return Some(Token::Comment(cmt_type, comment.iter().collect()));
            }
            loop {
                let Some(ch) = self.next_ch() else {
                    break;
                };
                comment.push(ch);
                if comment.ends_with(&";$ENDIF".chars().collect::<Vec<char>>()) {
                    self.if_level -= 1;
                    break;
                } else if comment.ends_with(&";$ELSE".chars().collect::<Vec<char>>()) {
                }
            }
            return Some(Token::Comment(cmt_type, comment.iter().collect()));
        }

        if comment.len() > "$DEFINE".len() && comment.iter().take("$DEFINE".len()).collect::<String>().to_ascii_uppercase() == "$DEFINE" {
            let reg = UserTypeRegistry::default();
            let input = comment.iter().skip("$DEFINE".len()).collect::<String>().trim().to_string();
            let (variable, value) = if input.chars().all(|c| c.is_ascii_alphabetic()) {
                (input, Constant::Boolean(true))
            } else {
                let mut parser = Parser::new(PathBuf::from("."), self.errors.clone(), &reg, &input, Encoding::Utf8, &Workspace::default());
                parser.next_token();
                if let Some(res) = parser.parse_statement() {
                    match res {
                        Statement::Let(expr) => {
                            let mut visitor = PreProcessorVisitor {
                                define_table: &self.define_table,
                                errors: self.errors.clone(),
                            };
                            let value = expr.get_value_expression().visit(&mut visitor);
                            if let Some(value) = value {
                                match value.get_type() {
                                    crate::executable::VariableType::Boolean => (expr.get_identifier().to_string(), Constant::Boolean(value.as_bool())),
                                    crate::executable::VariableType::Integer => {
                                        (expr.get_identifier().to_string(), Constant::Integer(value.as_int(), NumberFormat::Default))
                                    }
                                    _ => {
                                        self.errors
                                            .lock()
                                            .unwrap()
                                            .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                                        ("".to_string(), Constant::Boolean(false))
                                    }
                                }
                            } else {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                                ("".to_string(), Constant::Boolean(false))
                            }
                        }
                        _ => {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                            ("".to_string(), Constant::Boolean(false))
                        }
                    }
                } else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                    ("".to_string(), Constant::Boolean(false))
                }
            };

            if self.define_table.insert(Ascii::new(variable.clone()), value.clone()).is_some() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning(self.token_start..self.token_end, LexingErrorType::AlreadyDefined(variable.to_string()));
            }
            return Some(Token::Define(cmt_type, variable, value));
        }
        Some(Token::Comment(cmt_type, comment.iter().collect()))
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.token_start..self.token_end
    }

    fn check_eof(&mut self) {
        if self.if_level > 0 {
            self.errors
                .lock()
                .unwrap()
                .report_error(self.token_start..self.token_end, LexingErrorType::MissingEndIf);
        }
    }
    /*
    pub(crate) fn define(&mut self, variable: &str, value: Constant)  {
        self.define_table.insert(Ascii::new(variable.to_string()), value);
    }*/
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
