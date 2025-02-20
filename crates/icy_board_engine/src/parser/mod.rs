use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        Ast, AstNode, BlockStatement, CommentAstNode, Constant, DimensionSpecifier, FunctionDeclarationAstNode, FunctionImplementation, ParameterSpecifier,
        ProcedureDeclarationAstNode, ProcedureImplementation, Statement, VariableSpecifier,
    },
    compiler::user_data::{UserData, UserDataRegistry},
    executable::{FuncOpCode, FunctionDefinition, OpCode, StatementDefinition, VariableType},
    icy_board::{conferences::Conference, doors::Door, file_directory::FileDirectory, message_area::MessageArea},
};

use self::lexer::{Lexer, Spanned, Token};
use codepages::tables::CP437_TO_UNICODE;
use thiserror::Error;
use unicase::Ascii;

mod expression;
pub mod lexer;
pub mod statements;

#[cfg(test)]
mod declaration_tests;
#[cfg(test)]
mod expr_tests;
#[cfg(test)]
mod lexer_tests;
#[cfg(test)]
mod statement_tests;

#[derive(Error, Default, Debug, Clone, PartialEq)]
pub enum ParserErrorType {
    #[default]
    #[error("Unexpected error (should never happen)")]
    UnexpectedError,

    #[error("Too '{0}' from {1}")]
    InvalidInteger(String, String),

    #[error("Not enough arguments passed ({0}:{1}:{2})")]
    TooFewArguments(String, usize, i8),

    #[error("Too many arguments passed ({0}:{1}:{2})")]
    TooManyArguments(String, usize, i8),

    #[error("Invalid token encountered ({0})")]
    InvalidToken(Token),

    #[error("Missing open '(' found: {0}")]
    MissingOpenParens(Token),

    #[error("Missing close ')' found: {0}")]
    MissingCloseParens(Token),

    #[error("Missing close ']' found: {0}")]
    MissingCloseBracket(Token),

    #[error("Invalid token - label expected ({0})")]
    LabelExpected(Token),

    #[error("Invalid token - 'END' expected")]
    EndExpected,

    #[error("Expected identifier ({0})")]
    IdentifierExpected(Token),

    #[error("Expected '=' ({0})")]
    EqTokenExpected(Token),

    #[error("Expected 'TO' ({0})")]
    ToExpected(Token),

    #[error("Expected expression ({0})")]
    ExpressionExpected(Token),

    #[error("Expected statement")]
    StatementExpected,

    #[error("Too many dimensions for variable '{0}' (max 3)")]
    TooManyDimensions(usize),

    #[error("Invalid token '{0}' - 'CASE' expected")]
    CaseExpected(Token),

    #[error("Unexpected identifier ({0})")]
    UnknownIdentifier(String),

    #[error("Expected number ({0})")]
    NumberExpected(Token),

    #[error("Expected type ({0})")]
    TypeExpected(Token),

    #[error("Invalid declaration '{0}' expected either 'PROCEDURE' or 'FUNCTION'")]
    InvalidDeclaration(Token),

    #[error("VAR parameters are not allowed in functions")]
    VarNotAllowedInFunctions,

    #[error("No statements allowed outside of BEGIN...END block")]
    NoStatementsAllowedOutsideBlock,

    #[error("$USEFUNCS used after statements has no effect.")]
    UsefuncAfterStatement,

    #[error("No statements allowed after functions (use $USEFUNCS)")]
    NoStatementsAfterFunctions,

    #[error("EOL expected ({0})")]
    EolExpected(Token),

    #[error("Expected comma ({0})")]
    CommaExpected(Token),

    #[error("Expected 'THEN' ({0})")]
    ThenExpected(Token),

    #[error("Missing CASE keyword in SELECT CASE statement")]
    CaseExpectedAfterSelect,

    #[error("IF/WHILE requires a conditional expression to evaluate")]
    IfWhileConditionNotFound,

    #[error("Block start (IF/WHILE/FOR/SELECT) must come before block end statement")]
    BlockEndBeforeBlockStart,

    #[error("Can't declare a procudure for an existing statement ({0})")]
    StatementAlreadyDefined(Token),

    #[error("Can't declare a function for an existing function ({0})")]
    FunctionAlreadyDefined(Token),

    #[error("Version ({2}) not supported for statement ({0}:{1})")]
    StatementVersionNotSupported(OpCode, u16, u16),

    #[error("Version ({2}) not supported for function ({0:?}:{1})")]
    FunctionVersionNotSupported(FuncOpCode, u16, u16),

    #[error("Return with expression is only valid inside functions")]
    ReturnExpressionOutsideFunc,

    #[error("',' or '}}' expected")]
    CommaOrRBraceExpected,
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParserWarningType {
    #[error("$USEFUNCS is not valid there, ignoring.")]
    UsefuncsIgnored,
    #[error("$USEFUNCS already set, ignoring.")]
    UsefuncsAlreadySet,

    #[error("Next Identifier '{1}' should match next variable '{0}'")]
    NextIdentifierInvalid(unicase::Ascii<String>, Token),

    // old pplc parser allows that
    #[error("Procedure closed with 'ENDFUNC'")]
    ProcedureClosedWithEndFunc,

    // old pplc parser allows that
    #[error("Function closed with 'ENDPROC'")]
    FunctionClosedWithEndProc,
}

#[derive(Default)]
pub struct UserTypeRegistry {
    pub registered_types: HashMap<unicase::Ascii<String>, VariableType>,
    pub types: Vec<UserDataRegistry>,
}

pub const FIRST_ID: usize = 30;
pub const CONFERENCE_ID: usize = 30;
pub const MESSAGE_AREA_ID: usize = 31;
pub const FILE_DIRECTORY_ID: usize = 32;
pub const DOOR_ID: usize = 33;

impl UserTypeRegistry {
    pub fn icy_board_registry() -> Self {
        let mut reg = UserTypeRegistry::default();
        reg.register::<Conference>();
        reg.register::<MessageArea>();
        reg.register::<FileDirectory>();
        reg.register::<Door>();

        reg
    }

    pub fn get_type(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        if let Some(vt) = self.registered_types.get(identifier) {
            return Some(*vt);
        }
        None
    }

    pub fn register<'a, T: UserData>(&mut self) {
        let mut registry = UserDataRegistry::default();
        T::register_members(&mut registry);
        let id = self.types.len();
        self.registered_types
            .insert(unicase::Ascii::new(T::TYPE_NAME.to_string()), VariableType::UserData((FIRST_ID + id) as u8));
        self.types.push(registry);
    }

    pub fn get_type_from_id(&self, d: u8) -> Option<&UserDataRegistry> {
        let d = d as usize;
        if d < FIRST_ID || d >= self.types.len() + FIRST_ID {
            log::error!("Invalid user type id: {}", d);
            return None;
        }
        Some(&self.types[d - FIRST_ID])
    }
}

pub struct Parser<'a> {
    pub error_reporter: Arc<Mutex<ErrorReporter>>,

    pub type_registry: &'a UserTypeRegistry,
    lang_version: u16,
    pub require_user_variables: bool,

    cur_token: Option<Spanned<Token>>,
    lookahead_token: Option<Spanned<Token>>,
    type_hashes: &'static HashMap<unicase::Ascii<String>, VariableType>,
    lex: Lexer,

    // parser state
    use_funcs: bool,
    parsed_begin: bool,
    got_statement: bool,
    got_funcs: bool,
    in_function: bool,
}
lazy_static::lazy_static! {
    static ref PROC_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("PROC".to_string());
    static ref FUNC_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("FUNC".to_string());
}

impl<'a> Parser<'a> {
    pub fn new(
        file: PathBuf,
        error_reporter: Arc<Mutex<ErrorReporter>>,
        type_registry: &'a UserTypeRegistry,
        text: &str,
        encoding: Encoding,
        lang_version: u16,
    ) -> Self {
        let lex: Lexer = Lexer::new(file, lang_version, text, encoding, error_reporter.clone());
        Parser {
            error_reporter,
            lang_version,
            cur_token: None,
            lookahead_token: None,
            lex,
            require_user_variables: false,
            type_registry,
            type_hashes: if lang_version >= 400 {
                &TYPE_HASHES_400
            } else if lang_version >= 200 {
                &TYPE_HASHES
            } else {
                &TYPE_HASHES_100
            },
            use_funcs: false,
            parsed_begin: false,
            got_statement: false,
            got_funcs: false,
            in_function: false,
        }
    }

    pub fn get_cur_token(&self) -> Option<Token> {
        self.cur_token.as_ref().map(|token| token.token.clone())
    }

    /// Returns the next token of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn next_token(&mut self) -> Option<Spanned<Token>> {
        if let Some(token) = self.lookahead_token.take() {
            self.cur_token = Some(token);
            return self.cur_token.clone();
        }

        if let Some(token) = self.lex.next_token() {
            let is_else = token == Token::Else;
            let is_end = token == Token::Identifier(Ascii::new("END".to_string()));
            let is_case = token == Token::Case;
            self.cur_token = Some(Spanned::new(token, self.lex.span()));

            if is_else {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahed) = self.lex.next_token() {
                    if lookahed == Token::If {
                        self.cur_token = Some(Spanned::new(Token::ElseIf, start..self.lex.span().end));
                    } else {
                        self.lookahead_token = Some(Spanned::new(lookahed, end..self.lex.span().end));
                    }
                }
            } else if is_case {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahed) = self.lex.next_token() {
                    if lookahed == Token::Else {
                        self.cur_token = Some(Spanned::new(Token::Default, start..self.lex.span().end));
                    } else {
                        self.lookahead_token = Some(Spanned::new(lookahed, end..self.lex.span().end));
                    }
                }
            } else if is_end {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahed) = self.lex.next_token() {
                    match lookahed {
                        Token::If => {
                            self.cur_token = Some(Spanned::new(Token::EndIf, start..self.lex.span().end));
                        }
                        Token::While => {
                            self.cur_token = Some(Spanned::new(Token::EndWhile, start..self.lex.span().end));
                        }
                        Token::Select => {
                            self.cur_token = Some(Spanned::new(Token::EndSelect, start..self.lex.span().end));
                        }
                        Token::Loop => {
                            self.cur_token = Some(Spanned::new(Token::EndLoop, start..self.lex.span().end));
                        }
                        Token::For => {
                            self.cur_token = Some(Spanned::new(Token::Next, start..self.lex.span().end));
                        }
                        _ => {
                            let set_lookahad = if let Token::Identifier(id) = &lookahed {
                                if *id == *PROC_TOKEN {
                                    self.cur_token = Some(Spanned::new(Token::EndProc, end..self.lex.span().end));
                                    false
                                } else if *id == *FUNC_TOKEN {
                                    self.cur_token = Some(Spanned::new(Token::EndFunc, end..self.lex.span().end));
                                    false
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                            if set_lookahad {
                                self.lookahead_token = Some(Spanned::new(lookahed, end..self.lex.span().end));
                            }
                        }
                    }
                }
            }
        } else {
            self.cur_token = None;
        }
        self.cur_token.clone()
    }

    fn save_token_span(&self) -> std::ops::Range<usize> {
        if let Some(token) = &self.cur_token {
            token.span.clone()
        } else {
            0..0
        }
    }

    fn save_token(&self) -> Token {
        if let Some(token) = &self.cur_token {
            token.token.clone()
        } else {
            Token::Eol
        }
    }

    fn save_spanned_token(&self) -> Spanned<Token> {
        if let Some(token) = &self.cur_token {
            token.clone()
        } else {
            Spanned::new(Token::Eol, 0..0)
        }
    }

    fn report_error(&mut self, span: std::ops::Range<usize>, save_token: ParserErrorType) {
        self.error_reporter.lock().unwrap().report_error(span, save_token);
        while self.get_cur_token().is_some() && self.get_cur_token() != Some(Token::Eol) && !matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
            self.next_token();
        }
    }

    fn parse_ast_node(&mut self) -> Option<AstNode> {
        let cur_token = self.cur_token.as_ref()?;
        match cur_token.token {
            Token::Eol => {
                self.next_token();
            }
            Token::Function => {
                if let Some(func) = self.parse_function() {
                    self.got_funcs = true;
                    return Some(AstNode::Function(func));
                }
            }
            Token::Procedure => {
                if let Some(func) = self.parse_procedure() {
                    self.got_funcs = true;
                    return Some(AstNode::Procedure(func));
                }
            }
            Token::Declare => {
                if let Some(decl) = self.parse_declaration() {
                    return Some(decl);
                }
            }
            Token::UseFuncs(_, _) => {
                if self.use_funcs {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_warning(self.lex.span(), ParserWarningType::UsefuncsAlreadySet);
                }
                if self.got_statement {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(self.lex.span(), ParserErrorType::UsefuncAfterStatement);
                    self.next_token();
                    return None;
                }
                self.use_funcs = true;
                let cmt = self.save_spanned_token();
                self.next_token();
                return Some(AstNode::TopLevelStatement(Statement::Comment(CommentAstNode::new(cmt))));
            }
            _ => {
                let stmt = self.parse_statement();
                if let Some(stmt) = stmt {
                    if let Statement::Label(label) = &stmt {
                        if *label.get_label() == *statements::BEGIN_LABEL {
                            self.parsed_begin = true;
                        }
                    }

                    if self.use_funcs && !self.parsed_begin {
                        if matches!(stmt, Statement::Comment(_)) || matches!(stmt, Statement::VariableDeclaration(_)) {
                            return Some(AstNode::TopLevelStatement(stmt));
                        }

                        self.report_error(self.lex.span(), ParserErrorType::NoStatementsAllowedOutsideBlock);
                        return None;
                    }
                    if self.got_funcs && !self.use_funcs {
                        if matches!(stmt, Statement::Comment(_)) {
                            return Some(AstNode::TopLevelStatement(stmt));
                        }
                        self.report_error(stmt.get_span(), ParserErrorType::NoStatementsAfterFunctions);
                        return None;
                    }
                    if !self.got_statement && !matches!(stmt, Statement::VariableDeclaration(_)) && !matches!(stmt, Statement::Comment(_)) {
                        let mut main_block = vec![stmt];
                        loop {
                            let Some(cur_token) = &self.cur_token else {
                                break;
                            };
                            if cur_token.token == Token::Function || cur_token.token == Token::Procedure {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                main_block.push(stmt);
                            }
                        }
                        self.got_statement = true;
                        return Some(AstNode::Main(BlockStatement::empty(main_block)));
                    }
                    return Some(AstNode::TopLevelStatement(stmt));
                }
            }
        }
        None
    }
}

lazy_static::lazy_static! {

    static ref TYPE_HASHES_100: HashMap<unicase::Ascii<String>, VariableType> = {
        let mut m = HashMap::new();
        m.insert(unicase::Ascii::new("INTEGER".to_string()), VariableType::Integer);
        m.insert(unicase::Ascii::new("STRING".to_string()), VariableType::String);
        m.insert(unicase::Ascii::new("BOOLEAN".to_string()), VariableType::Boolean);
        m.insert(unicase::Ascii::new("DATE".to_string()), VariableType::Date);
        m.insert(unicase::Ascii::new("TIME".to_string()), VariableType::Time);
        m.insert(unicase::Ascii::new("MONEY".to_string()), VariableType::Money);
        m
    };

    static ref TYPE_HASHES: HashMap<unicase::Ascii<String>, VariableType> = {
        let mut m = HashMap::new();
        m.insert(unicase::Ascii::new("INTEGER".to_string()), VariableType::Integer);
        m.insert(unicase::Ascii::new("SDWORD".to_string()), VariableType::Integer);
        m.insert(unicase::Ascii::new("LONG".to_string()), VariableType::Integer);

        m.insert(unicase::Ascii::new("STRING".to_string()), VariableType::String);
        m.insert(unicase::Ascii::new("BIGSTR".to_string()), VariableType::BigStr);

        m.insert(unicase::Ascii::new("BOOLEAN".to_string()), VariableType::Boolean);

        m.insert(unicase::Ascii::new("DATE".to_string()), VariableType::Date);

        m.insert(unicase::Ascii::new("DDATE".to_string()), VariableType::DDate);

        m.insert(unicase::Ascii::new("EDATE".to_string()), VariableType::EDate);

        m.insert(unicase::Ascii::new("TIME".to_string()), VariableType::Time);

        m.insert(unicase::Ascii::new("MONEY".to_string()), VariableType::Money);

        m.insert(unicase::Ascii::new("WORD".to_string()), VariableType::Word);
        m.insert(unicase::Ascii::new("UWORD".to_string()), VariableType::Word);

        m.insert(unicase::Ascii::new("SWORD".to_string()), VariableType::SWord);
        m.insert(unicase::Ascii::new("INT".to_string()), VariableType::SWord);

        m.insert(unicase::Ascii::new("BYTE".to_string()), VariableType::Byte);
        m.insert(unicase::Ascii::new("UBYTE".to_string()), VariableType::Byte);

        m.insert(unicase::Ascii::new("UNSIGNED".to_string()), VariableType::Unsigned);
        m.insert(unicase::Ascii::new("DWORD".to_string()), VariableType::Unsigned);
        m.insert(unicase::Ascii::new("UDWORD".to_string()), VariableType::Unsigned);

        m.insert(unicase::Ascii::new("SBYTE".to_string()), VariableType::SByte);
        m.insert(unicase::Ascii::new("SHORT".to_string()), VariableType::SByte);

        m.insert(unicase::Ascii::new("REAL".to_string()), VariableType::Float);
        m.insert(unicase::Ascii::new("FLOAT".to_string()), VariableType::Float);

        m.insert(unicase::Ascii::new("DOUBLE".to_string()), VariableType::Double);
        m.insert(unicase::Ascii::new("DREAL".to_string()), VariableType::Double);
        m
    };


    static ref TYPE_HASHES_400: HashMap<unicase::Ascii<String>, VariableType> = {
        let mut m = HashMap::new();
        m.insert(unicase::Ascii::new("INTEGER".to_string()), VariableType::Integer);
        m.insert(unicase::Ascii::new("SDWORD".to_string()), VariableType::Integer);
        m.insert(unicase::Ascii::new("LONG".to_string()), VariableType::Integer);

        m.insert(unicase::Ascii::new("STRING".to_string()), VariableType::String);
        m.insert(unicase::Ascii::new("BIGSTR".to_string()), VariableType::BigStr);

        m.insert(unicase::Ascii::new("BOOLEAN".to_string()), VariableType::Boolean);

        m.insert(unicase::Ascii::new("DATE".to_string()), VariableType::Date);

        m.insert(unicase::Ascii::new("DDATE".to_string()), VariableType::DDate);

        m.insert(unicase::Ascii::new("EDATE".to_string()), VariableType::EDate);

        m.insert(unicase::Ascii::new("TIME".to_string()), VariableType::Time);

        m.insert(unicase::Ascii::new("MONEY".to_string()), VariableType::Money);

        m.insert(unicase::Ascii::new("WORD".to_string()), VariableType::Word);
        m.insert(unicase::Ascii::new("UWORD".to_string()), VariableType::Word);

        m.insert(unicase::Ascii::new("SWORD".to_string()), VariableType::SWord);
        m.insert(unicase::Ascii::new("INT".to_string()), VariableType::SWord);

        m.insert(unicase::Ascii::new("BYTE".to_string()), VariableType::Byte);
        m.insert(unicase::Ascii::new("UBYTE".to_string()), VariableType::Byte);

        m.insert(unicase::Ascii::new("UNSIGNED".to_string()), VariableType::Unsigned);
        m.insert(unicase::Ascii::new("DWORD".to_string()), VariableType::Unsigned);
        m.insert(unicase::Ascii::new("UDWORD".to_string()), VariableType::Unsigned);

        m.insert(unicase::Ascii::new("SBYTE".to_string()), VariableType::SByte);
        m.insert(unicase::Ascii::new("SHORT".to_string()), VariableType::SByte);

        m.insert(unicase::Ascii::new("REAL".to_string()), VariableType::Float);
        m.insert(unicase::Ascii::new("FLOAT".to_string()), VariableType::Float);

        m.insert(unicase::Ascii::new("DOUBLE".to_string()), VariableType::Double);
        m.insert(unicase::Ascii::new("DREAL".to_string()), VariableType::Double);

        m.insert(unicase::Ascii::new("MSGAREAID".to_string()), VariableType::MessageAreaID);
        m
    };

}

impl<'a> Parser<'a> {
    pub fn get_variable_type(&self) -> Option<VariableType> {
        if let Some(token) = &self.cur_token {
            if let Token::Identifier(id) = &token.token {
                if let Some(vt) = self.type_hashes.get(id) {
                    return Some(*vt);
                }
                if let Some(ut) = self.type_registry.get_type(id) {
                    return Some(ut);
                }
                None
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Returns the parse var info of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_var_info(&mut self, can_be_empty: bool) -> Option<VariableSpecifier> {
        if can_be_empty {
            if matches!(self.get_cur_token(), Some(Token::Comma)) || matches!(self.get_cur_token(), Some(Token::RPar)) {
                return None;
            }
        }
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        let mut dimensions = Vec::new();
        let mut leftpar_token = None;
        let mut rightpar_token = None;
        let is_lpar = matches!(self.get_cur_token(), Some(Token::LPar));
        if is_lpar || matches!(self.get_cur_token(), Some(Token::LBracket)) {
            leftpar_token = Some(self.save_spanned_token());
            self.next_token();
            let Some(Token::Const(Constant::Integer(_, _))) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::NumberExpected(self.save_token()));
                return None;
            };
            dimensions.push(DimensionSpecifier::new(self.save_spanned_token()));
            self.next_token();

            while let Some(Token::Comma) = &self.get_cur_token() {
                self.next_token();
                let Some(Token::Const(Constant::Integer(_, _))) = self.get_cur_token() else {
                    self.report_error(self.lex.span(), ParserErrorType::NumberExpected(self.save_token()));

                    return None;
                };
                dimensions.push(DimensionSpecifier::new(self.save_spanned_token()));
                self.next_token();
            }

            if dimensions.len() > 3 {
                self.report_error(self.lex.span(), ParserErrorType::TooManyDimensions(dimensions.len()));

                return None;
            }

            if is_lpar && !matches!(self.get_cur_token(), Some(Token::RPar)) || !is_lpar && !matches!(self.get_cur_token(), Some(Token::RBracket)) {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                return None;
            }
            rightpar_token = Some(self.save_spanned_token());
            self.next_token();
        } else if self.lang_version >= 350 {
            if let Some(Token::Eq) = self.get_cur_token() {
                let eq_token = self.next_token();
                let initializer = self.parse_expression();
                return Some(VariableSpecifier::new(identifier_token, None, dimensions, None, eq_token, initializer));
            }
        }

        Some(VariableSpecifier::new(identifier_token, leftpar_token, dimensions, rightpar_token, None, None))
    }

    /// Returns the parse function declaration of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_declaration(&mut self) -> Option<AstNode> {
        let declare_token = self.save_spanned_token();
        self.next_token();

        let is_function = if Some(Token::Procedure) == self.get_cur_token() {
            false
        } else if Some(Token::Function) == self.get_cur_token() {
            true
        } else {
            self.report_error(self.lex.span(), ParserErrorType::InvalidDeclaration(self.save_token()));
            return None;
        };
        let func_or_proc_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(identifier)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));

            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
            return None;
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));

                return None;
            }

            let mut var_token = None;
            if let Some(Token::Identifier(id)) = self.get_cur_token() {
                if id == Ascii::new("VAR".to_string()) {
                    if is_function {
                        self.report_error(self.lex.span(), ParserErrorType::VarNotAllowedInFunctions);
                    } else {
                        var_token = Some(self.save_spanned_token());
                    }
                    self.next_token();
                }
            }
            if let Some(var_type) = self.get_variable_type() {
                let type_token = self.save_spanned_token();
                self.next_token();
                let info = self.parse_var_info(true);
                parameters.push(ParameterSpecifier::new(var_token, type_token, var_type, info));
            } else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return None;
            }

            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
            }
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();
        if !is_function {
            self.check_eol();
            if StatementDefinition::get_statement_definition(&identifier).is_some() {
                self.report_error(identifier_token.span, ParserErrorType::StatementAlreadyDefined(self.save_token()));
                return None;
            }

            return Some(AstNode::ProcedureDeclaration(ProcedureDeclarationAstNode::new(
                declare_token,
                func_or_proc_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
            )));
        }
        if !FunctionDefinition::get_function_definitions(&identifier).is_empty() {
            self.report_error(identifier_token.span, ParserErrorType::FunctionAlreadyDefined(self.save_token()));
            return None;
        }
        let Some(return_type) = self.get_variable_type() else {
            self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
            return None;
        };
        let return_type_token = self.save_spanned_token();
        self.next_token();
        self.check_eol();
        Some(AstNode::FunctionDeclaration(FunctionDeclarationAstNode::new(
            declare_token,
            func_or_proc_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            return_type_token,
            return_type,
        )))
    }

    fn check_eol(&mut self) -> bool {
        if self.get_cur_token() != Some(Token::Eol) && !matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
            let err_token = self.save_spanned_token();
            self.next_token();
            self.report_error(err_token.span, ParserErrorType::EolExpected(err_token.token));
            false
        } else {
            true
        }
    }

    /// Returns the parse procedure of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_procedure(&mut self) -> Option<ProcedureImplementation> {
        if Some(Token::Procedure) == self.get_cur_token() {
            let procedure_token = self.save_spanned_token();
            self.next_token();

            let Some(Token::Identifier(_)) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));

                return None;
            };
            let identifier_token = self.save_spanned_token();
            self.next_token();
            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
                return None;
            }

            let leftpar_token = self.save_spanned_token();
            self.next_token();

            let mut parameters = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));

                    return None;
                }
                let mut var_token = None;
                if let Some(Token::Identifier(id)) = self.get_cur_token() {
                    if id.eq_ignore_ascii_case("VAR") {
                        var_token = Some(self.save_spanned_token());
                        self.next_token();
                    }
                }

                if let Some(var_type) = self.get_variable_type() {
                    let type_token = self.save_spanned_token();
                    self.next_token();

                    let info = self.parse_var_info(false);
                    parameters.push(ParameterSpecifier::new(var_token, type_token, var_type, info));
                } else {
                    self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                    return None;
                }

                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                }
            }
            let rightpar_token = self.save_spanned_token();
            self.next_token();

            self.skip_eol();

            let mut statements = Vec::new();

            while self.get_cur_token() != Some(Token::EndProc) && self.get_cur_token() != Some(Token::EndFunc) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            let endproc_token = self.save_spanned_token();
            if endproc_token.token == Token::EndFunc {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_warning(endproc_token.span.clone(), ParserWarningType::ProcedureClosedWithEndFunc);
            }
            self.next_token();

            return Some(ProcedureImplementation::new(
                usize::MAX,
                procedure_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
                statements.into_iter().flatten().collect(),
                endproc_token,
            ));
        }
        None
    }

    /// Returns the parse function of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_function(&mut self) -> Option<FunctionImplementation> {
        if Some(Token::Function) == self.get_cur_token() {
            let function_token = self.save_spanned_token();
            self.next_token();

            let Some(Token::Identifier(_)) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));

                return None;
            };
            let identifier_token = self.save_spanned_token();
            self.next_token();
            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
                return None;
            }

            let leftpar_token = self.save_spanned_token();
            self.next_token();

            let mut parameters = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));

                    return None;
                }
                if let Some(Token::Identifier(id)) = self.get_cur_token() {
                    if id == Ascii::new("VAR".to_string()) {
                        self.report_error(self.lex.span(), ParserErrorType::VarNotAllowedInFunctions);
                        self.next_token();
                    }
                }

                if let Some(var_type) = self.get_variable_type() {
                    let type_token = self.save_spanned_token();
                    self.next_token();

                    let info = self.parse_var_info(false);
                    parameters.push(ParameterSpecifier::new(None, type_token, var_type, info));
                } else {
                    self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                    return None;
                }

                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                }
            }
            let rightpar_token = self.save_spanned_token();
            self.next_token();

            let Some(return_type) = self.get_variable_type() else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return None;
            };
            let return_type_token = self.save_spanned_token();
            self.next_token();
            self.skip_eol();

            let mut statements = Vec::new();
            self.in_function = true;
            while self.get_cur_token() != Some(Token::EndProc) && self.get_cur_token() != Some(Token::EndFunc) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            self.in_function = false;

            let endfunc_token = self.save_spanned_token();
            if endfunc_token.token == Token::EndProc {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_warning(endfunc_token.span.clone(), ParserWarningType::FunctionClosedWithEndProc);
            }
            self.next_token();

            return Some(FunctionImplementation::new(
                usize::MAX,
                function_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
                return_type_token,
                return_type,
                statements.into_iter().flatten().collect(),
                endfunc_token.clone(),
            ));
        }
        None
    }
}

/// .
///
/// # Panics
///
/// Panics if .
pub fn parse_ast(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    version: u16,
) -> Ast {
    error_reporter.lock().unwrap().set_file_name(&file_name);
    let mut nodes = Vec::new();
    let mut parser = Parser::new(file_name.clone(), error_reporter, user_types, input, encoding, version);
    parser.next_token();
    parser.skip_eol();

    while parser.cur_token.is_some() {
        if let Some(node) = parser.parse_ast_node() {
            nodes.push(node);
        }
    }

    Ast {
        nodes,
        file_name,
        require_user_variables: parser.require_user_variables,
    }
}

pub struct ErrorContainer {
    pub error: Box<dyn std::error::Error + Send + Sync>,
    pub span: core::ops::Range<usize>,
    pub file_name: PathBuf,
}

#[derive(Default)]
pub struct ErrorReporter {
    cur_file: PathBuf,
    pub errors: Vec<ErrorContainer>,
    pub warnings: Vec<ErrorContainer>,
}

impl ErrorReporter {
    pub fn file_name(&self) -> &Path {
        &self.cur_file
    }
    pub fn set_file_name(&mut self, file_name: &Path) {
        self.cur_file = file_name.to_path_buf();
    }

    pub fn report_error<T: std::error::Error + 'static + Send + Sync>(&mut self, span: core::ops::Range<usize>, error: T) {
        self.errors.push(ErrorContainer {
            error: Box::new(error),
            span,
            file_name: self.cur_file.clone(),
        });
    }

    pub fn report_warning<T: std::error::Error + 'static + Send + Sync>(&mut self, span: core::ops::Range<usize>, warning: T) {
        self.warnings.push(ErrorContainer {
            error: Box::new(warning),
            span,
            file_name: self.cur_file.clone(),
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn report(&self) {}
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Detect,
    CP437,
    Utf8,
}

/// .
///
/// # Errors
///
/// This function will return an error if .
pub fn load_with_encoding<P: AsRef<Path>>(file_name: &P, encoding: Encoding) -> std::io::Result<String> {
    if encoding == Encoding::Detect {
        let src_data = fs::read(file_name)?;
        let src = codepages::tables::get_utf8(&src_data);
        return Ok(src);
    }
    let src_data = fs::read(file_name)?;
    let src = if encoding == Encoding::CP437 {
        let mut res = String::new();
        for b in src_data {
            res.push(CP437_TO_UNICODE[b as usize]);
        }
        res
    } else {
        codepages::tables::get_utf8(&src_data)
    };
    Ok(src)
}
