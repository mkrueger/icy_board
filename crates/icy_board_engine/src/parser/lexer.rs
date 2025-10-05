use crate::{
    ast::{
        Constant, Statement,
        constant::{BUILTIN_CONSTS, NumberFormat},
    },
    compiler::workspace::Workspace,
};
use core::fmt;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use unicase::Ascii;

use super::{Encoding, ErrorReporter, Parser, UserTypeRegistry, pre_processor_expr_visitor::PreProcessorVisitor};

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

#[derive(Debug, Clone)]
struct IfFrame {
    taken: bool,  // has any prior branch executed?
    active: bool, // emit tokens for this branch?
}

pub struct Lexer {
    lookup_table: &'static HashMap<unicase::Ascii<String>, Token>,
    define_table: HashMap<unicase::Ascii<String>, Constant>,
    lang_version: u16,
    text: Vec<char>,

    errors: Arc<Mutex<ErrorReporter>>,
    lexer_state: LexerState,
    token_start: usize,
    token_end: usize,
    if_stack: Vec<IfFrame>,

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
    pub fn new(_file: PathBuf, workspace: &Workspace, text: &str, _encoding: Encoding, errors: Arc<Mutex<ErrorReporter>>) -> Self {
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
            lang_version,
            define_table,
            text: text.chars().collect(),
            lexer_state: LexerState::AfterEol,
            errors,
            token_start: 0,
            token_end: 0,
            include_lexer: None,
            if_stack: Vec::new(),
        }
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.token_start..self.token_end
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
    /*
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

    */
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

    // Collect skipped inactive region starting at a false $IF or untaken $ELSEIF.
    // Includes lines until:
    //  * an activating $ELSE or $ELSEIF true branch (includes that directive line, then stops)
    //  * OR the matching $ENDIF (excludes that line so it becomes its own comment token)
    // Nested inactive $IF blocks are fully absorbed.
    fn collect_inactive_region(&mut self, mut collected: String, marker: CommentType) -> Token {
        if !collected.ends_with('\n') {
            collected.push('\n');
        }
        let mut nest = 0usize;

        loop {
            if self.token_end >= self.text.len() {
                break;
            }
            let line_start = self.token_end;
            let mut line_chars: Vec<char> = Vec::new();
            let mut first_non_ws: Option<char> = None;

            while let Some(ch) = self.next_ch() {
                line_chars.push(ch);
                if ch == '\n' {
                    break;
                }
                if first_non_ws.is_none() && !ch.is_whitespace() {
                    first_non_ws = Some(ch);
                }
            }
            if line_chars.is_empty() {
                break;
            }

            let line_str: String = line_chars.iter().collect();
            let is_comment_line = matches!(first_non_ws, Some(';') | Some('\'') | Some('*'));

            if !is_comment_line {
                collected.push_str(&line_str);
                continue;
            }

            // Extract directive body (after first marker)
            let marker_pos = line_str.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            let after_marker = &line_str[marker_pos + 1..];
            let upper = after_marker.trim_start().to_ascii_uppercase();

            // Nested IF
            if upper.starts_with("$IF ") || upper == "$IF" {
                nest += 1;
                collected.push_str(&line_str);
                continue;
            }

            // ENDIF
            if upper.starts_with("$ENDIF") {
                if nest == 0 {
                    // Stop BEFORE consuming this line: rewind so it becomes its own comment token.
                    self.token_end = line_start;
                    break;
                } else {
                    nest -= 1;
                    collected.push_str(&line_str);
                    continue;
                }
            }

            // At root level, sibling directives might activate or also be skipped
            if nest == 0 && (upper.starts_with("$ELSEIF") || upper.starts_with("$ELSE")) {
                let activating = if upper.starts_with("$ELSE") {
                    // $ELSE activates only if no branch taken yet
                    if let Some(frame) = self.if_stack.last() { !frame.taken } else { false }
                } else {
                    // $ELSEIF
                    if let Some(frame) = self.if_stack.last() {
                        if frame.taken {
                            false
                        } else {
                            // Evaluate expression (slice off "$ELSEIF")
                            let expr_src = &after_marker[after_marker.to_ascii_uppercase().find("$ELSEIF").unwrap() + "$ELSEIF".len()..];
                            self.eval_preproc_bool(expr_src)
                        }
                    } else {
                        false
                    }
                };

                if activating {
                    // Include this directive line then stop so following code is active.
                    collected.push_str(&line_str);
                    if let Some(f) = self.if_stack.last_mut() {
                        f.active = true;
                        f.taken = true;
                    }
                    break;
                } else {
                    // Still inactive, include and continue.
                    collected.push_str(&line_str);
                    continue;
                }
            }

            // Any other comment line in skipped region
            collected.push_str(&line_str);
        }

        // Return as a normal comment (NOT BlockComment) with the original marker type.
        Token::Comment(marker, collected)
    }

    // Adjust read_comment to call collect_inactive_region with marker and NOT produce BlockComment
    fn read_comment(&mut self, ch: char) -> Option<Token> {
        let cmt_type = match ch {
            ';' => CommentType::SingleLineSemicolon,
            '*' => CommentType::SingleLineStar,
            _ => CommentType::SingleLineQuote,
        };
        let mut comment = Vec::new();
        while let Some(ch2) = self.next_ch() {
            if ch2 == '\n' {
                break;
            }
            if comment.is_empty() && ch2 == '#' {
                return self.read_define();
            }
            comment.push(ch2);
        }
        self.lexer_state = LexerState::AfterEol;

        let raw = comment.iter().collect::<String>();
        let upper = raw.trim_start().to_ascii_uppercase();

        if upper.starts_with("$INCLUDE:") {
            return self.next_token();
        }
        if upper.starts_with("$USEFUNCS") {
            return Some(Token::UseFuncs(cmt_type, raw));
        }

        // $IF
        if upper.starts_with("$IF ") || upper == "$IF" {
            let expr_src = &raw[upper.find("$IF").unwrap() + 3..];
            let cond = self.eval_preproc_bool(expr_src);
            self.if_stack.push(IfFrame { taken: cond, active: cond });
            if !cond {
                let first_line = format!("{cmt_type}{raw}");
                return Some(self.collect_inactive_region(first_line, cmt_type));
            }
            return self.next_token();
        }

        // $ELSEIF
        if upper.starts_with("$ELSEIF") {
            if self.if_stack.is_empty() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::ElseIfWithoutIf);
                return self.next_token();
            }
            let already = self.if_stack.last().unwrap().taken;
            if already {
                if let Some(f) = self.if_stack.last_mut() {
                    f.active = false;
                }
                let first_line = format!("{cmt_type}{raw}");
                return Some(self.collect_inactive_region(first_line, cmt_type));
            } else {
                let expr_src = &raw[raw.to_ascii_uppercase().find("$ELSEIF").unwrap() + 7..];
                let cond = self.eval_preproc_bool(expr_src);
                if let Some(f) = self.if_stack.last_mut() {
                    f.active = cond;
                    if cond {
                        f.taken = true;
                    }
                }
                if !cond {
                    let first_line = format!("{cmt_type}{raw}");
                    return Some(self.collect_inactive_region(first_line, cmt_type));
                }
                return self.next_token();
            }
        }

        // $ELSE
        if upper.starts_with("$ELSE") {
            if self.if_stack.is_empty() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::ElseWithoutIf);
                return self.next_token();
            }
            let activate = {
                let f = self.if_stack.last().unwrap();
                !f.taken
            };
            if let Some(f) = self.if_stack.last_mut() {
                if activate {
                    f.active = true;
                    f.taken = true;
                } else {
                    f.active = false;
                }
            }
            if !activate {
                let first_line = format!("{cmt_type}{raw}");
                return Some(self.collect_inactive_region(first_line, cmt_type));
            }
            return self.next_token();
        }

        // $ENDIF: return as normal single-line comment, do NOT absorb earlier
        if upper.starts_with("$ENDIF") {
            if self.if_stack.pop().is_none() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::MissingEndIf);
            }
            return Some(Token::Comment(cmt_type, raw));
        }

        if upper.starts_with("$DEFINE") {
            let define_src = raw[upper.find("$DEFINE").unwrap() + 7..].trim();
            self.handle_define(define_src);
            return self.next_token();
        }

        Some(Token::Comment(cmt_type, raw))
    }
    fn eval_preproc_bool(&mut self, src: &str) -> bool {
        let expr = src.trim();
        if expr.is_empty() {
            return false;
        }
        let reg = UserTypeRegistry::default();
        let mut parser = Parser::new(PathBuf::from("."), self.errors.clone(), &reg, expr, Encoding::Utf8, &Workspace::default());
        parser.next_token();
        if let Some(ast) = parser.parse_expression() {
            let mut visitor = PreProcessorVisitor {
                define_table: &self.define_table,
                errors: self.errors.clone(),
            };
            if let Some(val) = ast.visit(&mut visitor) {
                return val.as_bool();
            }
        }
        false
    }

    fn handle_define(&mut self, input: &str) {
        // parse like before; reuse existing logic but moved out of read_comment
        let reg = UserTypeRegistry::default();
        if input.is_empty() {
            return;
        }
        if input.chars().all(|c| c.is_ascii_alphabetic()) {
            self.define_table.insert(Ascii::new(input.to_string()), Constant::Boolean(true));
            return;
        }
        let mut parser = Parser::new(PathBuf::from("."), self.errors.clone(), &reg, input, Encoding::Utf8, &Workspace::default());
        parser.next_token();
        if let Some(stmt) = parser.parse_statement() {
            if let Statement::Let(expr) = stmt {
                let mut visitor = PreProcessorVisitor {
                    define_table: &self.define_table,
                    errors: self.errors.clone(),
                };
                if let Some(val) = expr.get_value_expression().visit(&mut visitor) {
                    let (k, v) = match val.get_type() {
                        crate::executable::VariableType::Boolean => (expr.get_identifier().to_string(), Constant::Boolean(val.as_bool())),
                        crate::executable::VariableType::Integer => (expr.get_identifier().to_string(), Constant::Integer(val.as_int(), NumberFormat::Default)),
                        _ => {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                            return;
                        }
                    };
                    if self.define_table.insert(Ascii::new(k.clone()), v).is_some() {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_warning(self.token_start..self.token_end, LexingErrorType::AlreadyDefined(k));
                    }
                }
            }
        }
    }

    // In check_eof():
    fn check_eof(&mut self) {
        if !self.if_stack.is_empty() {
            self.errors
                .lock()
                .unwrap()
                .report_error(self.token_start..self.token_end, LexingErrorType::MissingEndIf);
        }
    }
    // ...existing code...

    pub fn next_token(&mut self) -> Option<Token> {
        // Handle include files first
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

        // Check if we're in an inactive conditional branch
        if !self.if_stack.is_empty() && !self.if_stack.last().unwrap().active {
            // Collect entire skipped block as a comment
            let mut skipped_content = String::new();

            // Skip tokens while in inactive conditional branch
            while !self.if_stack.is_empty() && !self.if_stack.last().unwrap().active {
                // We need to consume tokens until we find a directive that changes our state
                let ch;
                loop {
                    self.token_start = self.token_end;
                    if let Some(next_ch) = self.next_ch() {
                        if next_ch != ' ' && next_ch != '\t' {
                            ch = next_ch;
                            break;
                        }
                        skipped_content.push(next_ch);
                    } else {
                        self.check_eof();
                        // Return collected content as comment if we had any
                        if !skipped_content.is_empty() {
                            return Some(get_comment(skipped_content));
                        }
                        return None;
                    }
                }

                // Collect the character
                skipped_content.push(ch);

                // Only process comment lines that might contain directives
                if ch == '\'' || ch == ';' || (ch == '*' && self.lexer_state == LexerState::AfterEol) {
                    let mut comment = Vec::new();
                    while let Some(ch) = self.next_ch() {
                        skipped_content.push(ch);
                        if ch == '\n' {
                            self.lexer_state = LexerState::AfterEol;
                            break;
                        }
                        comment.push(ch);
                    }

                    let raw = comment.iter().collect::<String>();
                    let upper = raw.trim_start().to_ascii_uppercase();

                    // Check for directives that affect flow
                    if upper.starts_with("$ELSEIF") {
                        if self.if_stack.is_empty() {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(self.token_start..self.token_end, LexingErrorType::ElseIfWithoutIf);
                            continue;
                        }

                        let prior_taken = self.if_stack.last().unwrap().taken;
                        if !prior_taken {
                            let expr_src = &raw[raw.to_ascii_uppercase().find("$ELSEIF").unwrap() + 7..];
                            let cond = self.eval_preproc_bool(expr_src);
                            if let Some(frame) = self.if_stack.last_mut() {
                                frame.active = cond;
                                if cond {
                                    frame.taken = true;
                                    // Return the skipped block as a comment
                                    if !skipped_content.is_empty() {
                                        return Some(get_comment(skipped_content));
                                    }
                                    break; // Exit skip loop, we're now active
                                }
                            }
                        }
                    } else if upper.starts_with("$ELSE") {
                        if self.if_stack.is_empty() {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(self.token_start..self.token_end, LexingErrorType::ElseWithoutIf);
                            continue;
                        }
                        if let Some(frame) = self.if_stack.last_mut() {
                            if !frame.taken {
                                frame.active = true;
                                frame.taken = true;
                                // Return the skipped block as a comment
                                if !skipped_content.is_empty() {
                                    return Some(get_comment(skipped_content));
                                }
                                break; // Exit skip loop, we're now active
                            }
                        }
                    } else if upper.starts_with("$ENDIF") {
                        if self.if_stack.pop().is_none() {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(self.token_start..self.token_end, LexingErrorType::MissingEndIf);
                        }
                        // Return the skipped block as a comment
                        if !skipped_content.is_empty() {
                            return Some(get_comment(skipped_content));
                        }
                        break; // Exit skip loop, check next level
                    } else if upper.starts_with("$IF") {
                        // Nested IF while skipping - push an inactive frame
                        self.if_stack.push(IfFrame { taken: false, active: false });
                    }
                } else {
                    // Collect non-comment content in inactive regions
                    if ch == '\n' {
                        self.lexer_state = LexerState::AfterEol;
                    } else if ch == '\r' {
                        if let Some('\n') = self.next_ch() {
                            skipped_content.push('\n');
                            self.lexer_state = LexerState::AfterEol;
                        }
                    } else {
                        // Read the rest of the line/token
                        while let Some(ch) = self.next_ch() {
                            if ch == '\n' || ch == '\r' {
                                self.put_back();
                                break;
                            }
                            skipped_content.push(ch);
                        }
                    }
                }
            }
        }

        // Now process normal tokens (we're in an active region or no conditionals)
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
            '\'' | ';' => {
                return self.read_comment(ch);
            }
            '"' => {
                let mut string_result = String::new();
                loop {
                    let Some(sch) = self.next_ch() else {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(self.token_start..self.token_end, LexingErrorType::UnexpectedEOFInString);
                        return None;
                    };
                    if sch == '"' {
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
            '\\' => {
                // eol continuation
                let next = self.next_ch();
                if let Some('\r') = next {
                    if let Some('\n') = self.next_ch() {
                        return self.next_token();
                    }
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                }
                if let Some('\n') = next {
                    return self.next_token();
                }
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                return None;
            }
            '_' => {
                // eol continuation
                let next = self.next_ch();
                if let Some('\r') = next {
                    if let Some('\n') = self.next_ch() {
                        return self.next_token();
                    }
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                }
                if let Some('\n') = next {
                    return self.next_token();
                }
                return self.read_identifier();
            }
            '\r' => {
                return if let Some('\n') = self.next_ch() {
                    self.lexer_state = LexerState::AfterEol;
                    Some(Token::Eol)
                } else {
                    self.put_back();
                    self.lexer_state = LexerState::AfterEol;
                    Some(Token::Eol)
                };
            }
            '\n' => {
                self.lexer_state = LexerState::AfterEol;
                return Some(Token::Eol);
            }
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

                let identifier = unicase::Ascii::new(self.text[self.token_start + 1 + label_start..self.token_end].iter().collect::<String>());
                Some(Token::Label(identifier))
            }
            '(' => Some(Token::LPar),
            ')' => Some(Token::RPar),
            '[' => {
                if self.lang_version < 350 {
                    Some(Token::LPar)
                } else {
                    Some(Token::LBracket)
                }
            }
            ']' => {
                if self.lang_version < 350 {
                    Some(Token::RPar)
                } else {
                    Some(Token::RBracket)
                }
            }
            '{' => {
                if self.lang_version < 350 {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(self.token_start..self.token_end, LexingErrorType::DontUseBraces);
                    Some(Token::LPar)
                } else {
                    Some(Token::LBrace)
                }
            }
            '}' => {
                if self.lang_version < 350 {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(self.token_start..self.token_end, LexingErrorType::DontUseBraces);
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
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(self.token_start..self.token_end, LexingErrorType::PowWillGetRemoved);
                    Some(Token::PoW)
                } else {
                    if self.lang_version >= 350 && next == Some('=') {
                        return Some(Token::MulAssign);
                    }
                    self.put_back();
                    Some(Token::Mul)
                }
            }
            '/' => {
                if self.lang_version >= 350 {
                    let next = self.next_ch();
                    if next == Some('=') {
                        return Some(Token::DivAssign);
                    }
                    self.put_back();
                }
                Some(Token::Div)
            }
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
            }
            '&' => {
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
            }
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
            }
            '!' => {
                let next = self.next_ch();
                if let Some('=') = next {
                    Some(Token::NotEq)
                } else {
                    self.put_back();
                    Some(Token::Not)
                }
            }
            '@' => {
                let ch = self.next_ch();
                if Some('X') != ch && Some('x') != ch {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                }
                let Some(first) = self.next_ch() else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                };
                let Some(second) = self.next_ch() else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                };
                if !first.is_ascii_hexdigit() || !second.is_ascii_hexdigit() {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                }
                Some(Token::Const(Constant::Integer(
                    conv_hex(first) * 16 + conv_hex(second),
                    NumberFormat::ColorCode,
                )))
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
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
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
            }
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
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                    return None;
                }
            }
            _ => {
                if ch.is_ascii_alphabetic() || ch == '_' {
                    return self.read_identifier();
                }

                if ch.is_ascii_digit() {
                    self.lexer_state = LexerState::BeyondEOL;
                    // ... (rest of numeric handling unchanged) ...
                    let start = self.token_start;
                    let mut cur_ch = ch;
                    loop {
                        let Some(ch) = self.next_ch() else {
                            break;
                        };
                        cur_ch = ch;

                        match ch {
                            '.' => {
                                break;
                            }
                            'D' | 'd' => {
                                let r = self.text[start..self.token_end - 1].iter().collect::<String>().parse::<i32>();
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Dec)));
                                    }
                                    Err(r) => {
                                        self.errors.lock().unwrap().report_warning(
                                            self.token_start..self.token_end,
                                            LexingErrorType::InvalidInteger(
                                                r.to_string(),
                                                self.text[self.token_start..self.token_end].iter().collect::<String>(),
                                            ),
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
                                            LexingErrorType::InvalidInteger(
                                                r.to_string(),
                                                self.text[self.token_start..self.token_end].iter().collect::<String>(),
                                            ),
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
                                            LexingErrorType::InvalidInteger(
                                                r.to_string(),
                                                self.text[self.token_start..self.token_end].iter().collect::<String>(),
                                            ),
                                        );
                                        return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
                                    }
                                }
                            }
                            'B' | 'b' => {
                                if let Some(ch) = self.next_ch() {
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
                                            LexingErrorType::InvalidInteger(
                                                r.to_string(),
                                                self.text[self.token_start..self.token_end].iter().collect::<String>(),
                                            ),
                                        );

                                        return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
                                    }
                                }
                            }
                            _ => {}
                        }
                        if !ch.is_ascii_hexdigit() {
                            self.put_back();
                            break;
                        }
                    }
                    let mut end = self.token_end;
                    if cur_ch == '.' {
                        let mut found_dot_dot = false;
                        if let Some(ch) = self.next_ch() {
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
                                        LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>()),
                                    );
                                    return Some(Token::Const(Constant::Double(-1.0)));
                                }
                            }
                        }
                    }

                    let r = self.text[start..end].iter().collect::<String>().parse::<i64>();
                    match r {
                        Ok(i) => {
                            if i32::try_from(i).is_ok() {
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
                                LexingErrorType::InvalidInteger(r.to_string(), self.text[self.token_start..self.token_end].iter().collect::<String>()),
                            );
                            return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
                        }
                    }
                    return Some(Token::Const(Constant::Integer(-1, NumberFormat::Default)));
                }

                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::InvalidToken);
                return None;
            }
        };

        self.lexer_state = LexerState::BeyondEOL;
        state
    }

    // ...existing code...
    /*
    pub(crate) fn define(&mut self, variable: &str, value: Constant)  {
        self.define_table.insert(Ascii::new(variable.to_string()), value);
    }*/
}

fn get_comment(skipped_content: String) -> Token {
    let comment_type = if skipped_content.starts_with(';') {
        CommentType::SingleLineSemicolon
    } else if skipped_content.starts_with('\'') {
        CommentType::SingleLineQuote
    } else if skipped_content.starts_with('*') {
        CommentType::SingleLineStar
    } else {
        CommentType::SingleLineSemicolon
    };

    Token::Comment(comment_type, skipped_content)
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
