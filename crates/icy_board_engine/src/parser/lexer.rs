use crate::{
    ast::{
        Constant,
        constant::{BUILTIN_CONSTS, NumberFormat},
    },
    compiler::workspace::Workspace,
    executable::{SUPPORTED_PPL_LANGUAGE_VERSIONS, VariableValue},
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

    #[error("Invalid $DEFINE directive: '{0}'")]
    InvalidDefine(String),

    #[error("Invalid pre processor expression: '{0}'")]
    InvalidPreProcessorExpression(String),

    #[error("Already defined ({0})")]
    AlreadyDefined(String),

    #[error("$ELSE without $IF")]
    ElseWithoutIf,

    #[error("$ELSEIF without $IF")]
    ElseIfWithoutIf,

    #[error("Missing $ENDIF")]
    MissingEndIf,

    #[error("$ENDIF without $IF")]
    EndIfWithoutIf,

    #[error("Undefined pre processor token ({0})")]
    UndefinedPreProcessorToken(String),

    #[error("Invalid $LANGVERSION '{0}', valid values are {SUPPORTED_PPL_LANGUAGE_VERSIONS:?}")]
    InvalidLanguageVersion(String),

    #[error("$LANGVERSION has to come before everything else in a file")]
    LanguageVersionMustComeFirst,
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

    Type,
    EndType,
    Enum,
    EndEnum,
    /// The keyword, as opposed to `Const` which carries a value.
    ConstDecl,
    Begin,
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
            Token::Type => write!(f, "TYPE"),
            Token::EndType => write!(f, "ENDTYPE"),
            Token::Enum => write!(f, "ENUM"),
            Token::EndEnum => write!(f, "ENDENUM"),
            Token::Begin => write!(f, "BEGIN"),
            Token::ConstDecl => write!(f, "CONST"),
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
    taken: bool,
}

fn directive_len(upper: &str, directive: &str) -> Option<usize> {
    let rest = upper.strip_prefix(directive)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(directive.len())
}

/// A `;$LANGVERSION` line. It is read before the first token, so it can pick the
/// keywords the rest of the file is lexed with.
pub struct LanguageVersionDirective {
    pub version: Option<u16>,
    pub value: String,
    pub span: core::ops::Range<usize>,
    /// Nothing but comments and blank lines come before it.
    pub in_header: bool,
}

/// Every `;$LANGVERSION` a source has, the misplaced ones included, so a caller can
/// report them.
pub fn scan_language_version_directives(text: &str) -> Vec<LanguageVersionDirective> {
    let mut directives = Vec::new();
    let mut in_header = true;
    let mut offset = 0;

    for line in text.split('\n') {
        let line_len = line.chars().count();
        let trimmed = line.trim_start();
        let indent = line_len - trimmed.chars().count();

        if let Some(body) = trimmed.strip_prefix([';', '\'', '*']) {
            let body = body.trim_start();
            let upper = body.to_ascii_uppercase();
            if let Some(len) = directive_len(&upper, "$LANGVERSION") {
                let value = body[len..].trim().to_string();
                let version = value.parse::<u16>().ok().filter(|v| SUPPORTED_PPL_LANGUAGE_VERSIONS.contains(v));
                directives.push(LanguageVersionDirective {
                    version,
                    value,
                    span: offset + indent..offset + line_len,
                    in_header,
                });
            }
        } else if !trimmed.trim_end().is_empty() {
            in_header = false;
        }

        offset += line_len + 1;
    }

    directives
}

/// The language version a source declares for itself.
pub fn scan_language_version(text: &str) -> Option<u16> {
    scan_language_version_directives(text).into_iter().find(|d| d.in_header).and_then(|d| d.version)
}

fn elseif_directive_len(upper: &str) -> Option<usize> {
    directive_len(upper, "$ELSEIF").or_else(|| directive_len(upper, "$ELIF"))
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
    /// True while the last read ran into the end of the file, so that putting it
    /// back does not step over a character that was never read.
    read_past_end: bool,
    if_stack: Vec<IfFrame>,

    include_lexer: Option<Box<Lexer>>,
}

/// A word the language reserves, and the version that reserved it.
///
/// A source below that version may still use the name for a variable, so this is
/// what decides whether a word is a keyword or an identifier.
pub struct Keyword {
    pub name: &'static str,
    pub token: Token,
    pub since: u16,
}

/// Every reserved word of the language. The single place that says since when.
#[rustfmt::skip]
pub const KEYWORDS: &[Keyword] = &[
    Keyword { name: "if",        token: Token::If,        since: 100 },
    Keyword { name: "let",       token: Token::Let,       since: 100 },
    Keyword { name: "while",     token: Token::While,     since: 100 },
    Keyword { name: "endwhile",  token: Token::EndWhile,  since: 100 },
    Keyword { name: "else",      token: Token::Else,      since: 100 },
    Keyword { name: "elseif",    token: Token::ElseIf,    since: 100 },
    Keyword { name: "endif",     token: Token::EndIf,     since: 100 },
    Keyword { name: "for",       token: Token::For,       since: 100 },
    Keyword { name: "next",      token: Token::Next,      since: 100 },
    Keyword { name: "endfor",    token: Token::Next,      since: 100 },
    Keyword { name: "break",     token: Token::Break,     since: 100 },
    Keyword { name: "continue",  token: Token::Continue,  since: 100 },
    Keyword { name: "return",    token: Token::Return,    since: 100 },
    Keyword { name: "gosub",     token: Token::Gosub,     since: 100 },
    Keyword { name: "goto",      token: Token::Goto,      since: 100 },

    Keyword { name: "select",    token: Token::Select,    since: 200 },
    Keyword { name: "case",      token: Token::Case,      since: 200 },
    Keyword { name: "default",   token: Token::Default,   since: 200 },
    Keyword { name: "endselect", token: Token::EndSelect, since: 200 },

    Keyword { name: "declare",   token: Token::Declare,   since: 300 },
    Keyword { name: "function",  token: Token::Function,  since: 300 },
    Keyword { name: "procedure", token: Token::Procedure, since: 300 },
    Keyword { name: "endproc",   token: Token::EndProc,   since: 300 },
    Keyword { name: "endfunc",   token: Token::EndFunc,   since: 300 },

    Keyword { name: "repeat",    token: Token::Repeat,    since: 350 },
    Keyword { name: "until",     token: Token::Until,     since: 350 },
    Keyword { name: "loop",      token: Token::Loop,      since: 350 },
    Keyword { name: "endloop",   token: Token::EndLoop,   since: 350 },
    // CONST and ENUM are gone before anything is emitted, so they cost an old runtime nothing.
    Keyword { name: "const",     token: Token::ConstDecl, since: 350 },
    Keyword { name: "enum",      token: Token::Enum,      since: 350 },
    Keyword { name: "endenum",   token: Token::EndEnum,   since: 350 },

    Keyword { name: "type",      token: Token::Type,      since: 400 },
    Keyword { name: "endtype",   token: Token::EndType,   since: 400 },
    Keyword { name: "begin",     token: Token::Begin,     since: 400 },
];

/// One table per version that reserves a word, in ascending order.
static TOKEN_LOOKUP_TABLES: std::sync::LazyLock<Vec<(u16, HashMap<unicase::Ascii<String>, Token>)>> = std::sync::LazyLock::new(|| {
    let mut versions: Vec<u16> = KEYWORDS.iter().map(|keyword| keyword.since).collect();
    versions.sort_unstable();
    versions.dedup();

    versions
        .into_iter()
        .map(|version| {
            let mut m: HashMap<unicase::Ascii<String>, Token> = KEYWORDS
                .iter()
                .filter(|keyword| keyword.since <= version)
                .map(|keyword| (unicase::Ascii::new(keyword.name.to_string()), keyword.token.clone()))
                .collect();

            for c in &BUILTIN_CONSTS {
                m.insert(unicase::Ascii::new(c.name.to_string()), Token::Const(Constant::Builtin(c)));
            }
            (version, m)
        })
        .collect()
});

/// The words a source of this language version may not use as a name.
fn token_lookup_table(lang_version: u16) -> &'static HashMap<unicase::Ascii<String>, Token> {
    let tables = &*TOKEN_LOOKUP_TABLES;
    let index = tables.partition_point(|(version, _)| *version <= lang_version).saturating_sub(1);
    &tables[index].1
}

impl Lexer {
    pub fn new(_file: PathBuf, workspace: &Workspace, text: &str, _encoding: Encoding, errors: Arc<Mutex<ErrorReporter>>) -> Self {
        let mut lang_version = workspace.language_version();
        let mut declared = false;
        for directive in scan_language_version_directives(text) {
            if !directive.in_header || declared {
                errors
                    .lock()
                    .unwrap()
                    .report_error(directive.span, LexingErrorType::LanguageVersionMustComeFirst);
                continue;
            }
            declared = true;
            match directive.version {
                Some(version) => lang_version = version,
                None => errors
                    .lock()
                    .unwrap()
                    .report_error(directive.span, LexingErrorType::InvalidLanguageVersion(directive.value)),
            }
        }

        let mut define_table = HashMap::new();
        define_table.insert(Ascii::new("VERSION".into()), Constant::String(workspace.package.version.to_string()));
        define_table.insert(Ascii::new("LANGVERSION".into()), Constant::Integer(lang_version as i32, NumberFormat::Default));
        define_table.insert(
            Ascii::new("RUNTIME".into()),
            Constant::Integer(workspace.runtime() as i32, NumberFormat::Default),
        );

        let mut lexer = Self {
            lookup_table: token_lookup_table(lang_version),
            lang_version,
            define_table,
            text: text.chars().collect(),
            lexer_state: LexerState::AfterEol,
            errors,
            token_start: 0,
            token_end: 0,
            read_past_end: false,
            include_lexer: None,
            if_stack: Vec::new(),
        };
        if let Some(defines) = workspace.compiler.as_ref().and_then(|compiler| compiler.defines.as_ref()) {
            for define in defines {
                lexer.handle_define(define);
            }
        }
        lexer
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.token_start..self.token_end
    }

    /// The version the file declared for itself, or the one the workspace asked for.
    pub fn lang_version(&self) -> u16 {
        self.lang_version
    }

    pub fn get_define(&self, key: &str) -> Option<&Constant> {
        self.define_table.get(&Ascii::new(key.to_string()))
    }

    #[inline]
    fn next_ch(&mut self) -> Option<char> {
        if self.token_end >= self.text.len() {
            self.read_past_end = true;
            None
        } else {
            let t = self.text[self.token_end];
            // Some files take that as end of file char.
            if t == '\x1A' {
                self.read_past_end = true;
                return None;
            }
            self.token_end += 1;
            self.read_past_end = false;
            Some(t)
        }
    }

    #[inline]
    fn put_back(&mut self) {
        if self.read_past_end {
            self.read_past_end = false;
            return;
        }
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

                    // An enum member is written with it, so it is needed from 350 on.
                    if self.lang_version >= 350 {
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
                                let literal = self.text[start..self.token_end - 1].iter().collect::<String>();
                                let r = i32::from_str_radix(&literal, 16);
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Hex)));
                                    }
                                    Err(r) => {
                                        if let Ok(i) = u64::from_str_radix(&literal, 16) {
                                            return Some(Token::Const(Constant::Unsigned(i, NumberFormat::Hex)));
                                        }
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
                                return Some(Token::Const(Constant::Unsigned(i as u64, NumberFormat::Default)));
                            }
                        }
                        Err(r) => {
                            let r2 = self.text[start..end].iter().collect::<String>().parse::<u64>();
                            if let Ok(i) = r2 {
                                return Some(Token::Const(Constant::Unsigned(i, NumberFormat::Default)));
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
        while let Some(ch) = self.next_ch() {
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
        if !open_bracket && let Some(token) = self.lookup_table.get(&identifier) {
            return Some(token.clone());
        }
        Some(Token::Identifier(identifier))
    }

    fn read_define(&mut self) -> Option<Token> {
        let mut define = String::new();
        while let Some(ch) = self.next_ch() {
            if !char::is_alphanumeric(ch) {
                self.put_back();
                break;
            }
            define.push(ch);
        }

        if let Some(value) = self.define_table.get(&Ascii::new(define.clone())) {
            return Some(Token::Const(value.clone()));
        }
        // Returning None here would look like end of file and drop the rest of the source.
        self.errors
            .lock()
            .unwrap()
            .report_error(self.token_start..self.token_end, LexingErrorType::UndefinedPreProcessorToken(define));
        self.next_token()
    }

    // Collect a skipped region starting at a false $IF, an untaken $ELSEIF or $ELSE.
    // Collecting stops after:
    //  * an activating $ELSE or true $ELSEIF, so the code behind it is lexed again, or
    //  * the matching $ENDIF, which also pops the frame, or
    //  * end of file, which leaves the frame for check_eof to report.
    // Nested blocks are absorbed whole. The text keeps the source verbatim apart from
    // the leading comment marker, which the returned CommentType stands for.
    fn collect_inactive_region(&mut self, mut collected: String, marker: CommentType) -> Token {
        if !collected.ends_with('\n') {
            collected.push('\n');
        }
        let mut nest = 0usize;

        loop {
            if self.token_end >= self.text.len() {
                break;
            }
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
            let is_comment_line = matches!(first_non_ws, Some(';' | '\'' | '*'));

            if !is_comment_line {
                collected.push_str(&line_str);
                continue;
            }

            // Extract directive body (after first marker)
            let marker_pos = line_str.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            let after_marker = &line_str[marker_pos + 1..];
            let upper = after_marker.trim_start().to_ascii_uppercase();

            // Nested IF
            if directive_len(&upper, "$IF").is_some() {
                nest += 1;
                collected.push_str(&line_str);
                continue;
            }

            // ENDIF
            if directive_len(&upper, "$ENDIF").is_some() {
                collected.push_str(&line_str);
                if nest == 0 {
                    self.if_stack.pop();
                    break;
                }
                nest -= 1;
                continue;
            }

            // At root level, sibling directives might activate or also be skipped
            let elseif_len = elseif_directive_len(&upper);
            if nest == 0 && (elseif_len.is_some() || directive_len(&upper, "$ELSE").is_some()) {
                let activating = if let Some(len) = elseif_len {
                    let already_taken = self.if_stack.last().is_some_and(|frame| frame.taken);
                    let expr_src = &after_marker[after_marker.find('$').unwrap() + len..];
                    let condition = self.eval_preproc_bool(expr_src);
                    !already_taken && condition
                } else {
                    // $ELSE activates only if no branch taken yet
                    if let Some(frame) = self.if_stack.last() { !frame.taken } else { false }
                };

                collected.push_str(&line_str);
                if activating {
                    if let Some(f) = self.if_stack.last_mut() {
                        f.taken = true;
                    }
                    break;
                }
                continue;
            }

            // Any other comment line in skipped region
            collected.push_str(&line_str);
        }

        // Return as a normal comment (NOT BlockComment) with the original marker type.
        Token::Comment(marker, collected)
    }

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
        if let Some(len) = directive_len(&upper, "$IF") {
            let expr_src = &raw[raw.find('$').unwrap() + len..];
            let cond = self.eval_preproc_bool(expr_src);
            self.if_stack.push(IfFrame { taken: cond });
            if !cond {
                return Some(self.collect_inactive_region(raw, cmt_type));
            }
            return self.next_token();
        }

        // $ELSEIF
        if let Some(len) = elseif_directive_len(&upper) {
            if self.if_stack.is_empty() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::ElseIfWithoutIf);
                return self.next_token();
            }
            let already = self.if_stack.last().unwrap().taken;
            let expr_src = &raw[raw.find('$').unwrap() + len..];
            if already {
                self.eval_preproc_bool(expr_src);
                return Some(self.collect_inactive_region(raw, cmt_type));
            }
            let cond = self.eval_preproc_bool(expr_src);
            if let Some(f) = self.if_stack.last_mut()
                && cond
            {
                f.taken = true;
            }
            if !cond {
                return Some(self.collect_inactive_region(raw, cmt_type));
            }
            return self.next_token();
        }

        // $ELSE
        if directive_len(&upper, "$ELSE").is_some() {
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
            if let Some(f) = self.if_stack.last_mut()
                && activate
            {
                f.taken = true;
            }
            if !activate {
                return Some(self.collect_inactive_region(raw, cmt_type));
            }
            return self.next_token();
        }

        // $ENDIF: return as normal single-line comment, do NOT absorb earlier
        if directive_len(&upper, "$ENDIF").is_some() {
            if self.if_stack.pop().is_none() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(self.token_start..self.token_end, LexingErrorType::EndIfWithoutIf);
            }
            return Some(Token::Comment(cmt_type, raw));
        }

        if let Some(len) = directive_len(&upper, "$DEFINE") {
            let define_src = raw[raw.find('$').unwrap() + len..].trim();
            self.handle_define(define_src);
            return self.next_token();
        }

        Some(Token::Comment(cmt_type, raw))
    }
    fn parse_preproc_value(&self, src: &str) -> Result<Option<VariableValue>, ()> {
        let expr = src.trim();
        if expr.is_empty() {
            return Err(());
        }
        let reg = UserTypeRegistry::default();
        let parse_errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let mut parser = Parser::new(PathBuf::from("."), parse_errors.clone(), &reg, expr, Encoding::Utf8, &Workspace::default());
        parser.next_token();
        let Some(expression) = parser.parse_expression() else {
            return Err(());
        };
        if parser.get_cur_token().is_some() || !parse_errors.lock().unwrap().errors.is_empty() {
            return Err(());
        }
        let mut visitor = PreProcessorVisitor {
            define_table: &self.define_table,
            errors: parse_errors.clone(),
        };
        let value = expression.visit(&mut visitor);
        if parse_errors.lock().unwrap().errors.is_empty() { Ok(value) } else { Err(()) }
    }

    fn eval_preproc_bool(&mut self, src: &str) -> bool {
        match self.parse_preproc_value(src) {
            Ok(Some(value)) => value.as_bool(),
            Ok(None) => false,
            Err(()) => {
                self.errors.lock().unwrap().report_error(
                    self.token_start..self.token_end,
                    LexingErrorType::InvalidPreProcessorExpression(src.trim().to_string()),
                );
                false
            }
        }
    }

    fn handle_define(&mut self, input: &str) {
        let input = input.trim();
        if input.is_empty() {
            self.errors
                .lock()
                .unwrap()
                .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefine("missing name".to_string()));
            return;
        }

        let (name, value) = if let Some((name, value)) = input.split_once('=') {
            (name.trim(), Some(value.trim()))
        } else {
            (input, None)
        };
        let mut chars = name.chars();
        let valid_name = chars.next().is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_') && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
        if !valid_name {
            self.errors
                .lock()
                .unwrap()
                .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefine(input.to_string()));
            return;
        }

        let value = if let Some(value) = value {
            match self.parse_preproc_value(value) {
                Ok(Some(value)) => match value.get_type() {
                    crate::executable::VariableType::Boolean => Constant::Boolean(value.as_bool()),
                    crate::executable::VariableType::Integer => Constant::Integer(value.as_int(), NumberFormat::Default),
                    _ => {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                        return;
                    }
                },
                Ok(None) => {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefineValue(input.to_string()));
                    return;
                }
                Err(()) => {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.token_start..self.token_end, LexingErrorType::InvalidDefine(input.to_string()));
                    return;
                }
            }
        } else {
            Constant::Boolean(true)
        };

        if self.define_table.insert(Ascii::new(name.to_string()), value).is_some() {
            self.errors
                .lock()
                .unwrap()
                .report_warning(self.token_start..self.token_end, LexingErrorType::AlreadyDefined(name.to_string()));
        }
    }

    fn check_eof(&mut self) {
        if !self.if_stack.is_empty() {
            self.errors
                .lock()
                .unwrap()
                .report_error(self.token_start..self.token_end, LexingErrorType::MissingEndIf);
        }
    }
    pub fn next_token(&mut self) -> Option<Token> {
        // Handle include files first
        if let Some(lexer) = &mut self.include_lexer {
            let result = lexer.next_token();
            if let Some(token) = result {
                return Some(token);
            }
            self.check_eof();
            self.include_lexer = None;
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
                while let Some(ch) = self.next_ch() {
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
                    if self.lang_version >= 350 && next == Some('=') {
                        return Some(Token::AndAssign);
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
                    if self.lang_version >= 350 && next == Some('=') {
                        return Some(Token::OrAssign);
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
                    if self.lang_version >= 350 {
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
                    while let Some(ch) = self.next_ch() {
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
                                let literal = self.text[start..self.token_end - 1].iter().collect::<String>();
                                let r = i32::from_str_radix(&literal, 16);
                                match r {
                                    Ok(i) => {
                                        return Some(Token::Const(Constant::Integer(i, NumberFormat::Hex)));
                                    }
                                    Err(r) => {
                                        if let Ok(i) = u64::from_str_radix(&literal, 16) {
                                            return Some(Token::Const(Constant::Unsigned(i, NumberFormat::Hex)));
                                        }
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
                                return Some(Token::Const(Constant::Unsigned(i as u64, NumberFormat::Default)));
                            }
                        }
                        Err(r) => {
                            let r2 = self.text[start..end].iter().collect::<String>().parse::<u64>();
                            if let Ok(i) = r2 {
                                return Some(Token::Const(Constant::Unsigned(i, NumberFormat::Default)));
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
