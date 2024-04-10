use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        Ast, AstNode, BlockStatement, CommentAstNode, Constant, DimensionSpecifier,
        FunctionDeclarationAstNode, FunctionImplementation, ParameterSpecifier,
        ProcedureDeclarationAstNode, ProcedureImplementation, Statement, VariableSpecifier,
    },
    executable::{FuncOpCode, FunctionDefinition, OpCode, StatementDefinition, VariableType},
    tables::CP437_TO_UNICODE,
};

use self::lexer::{Lexer, Spanned, Token};
use thiserror::Error;
use unicase::Ascii;

mod expression;
pub mod lexer;
mod statements;

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

pub struct Parser {
    pub errors: Arc<Mutex<ErrorRepoter>>,
    version: u16,
    pub require_user_variables: bool,

    cur_token: Option<Spanned<Token>>,
    lookahead_token: Option<Spanned<Token>>,

    lex: Lexer,

    // parser state
    use_funcs: bool,
    parsed_begin: bool,
    got_statement: bool,
    got_funcs: bool,
}
lazy_static::lazy_static! {
    static ref PROC_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("PROC".to_string());
    static ref FUNC_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("FUNC".to_string());
}

impl Parser {
    pub fn new(file: PathBuf, text: &str, encoding: Encoding, version: u16) -> Self {
        let errors = Arc::new(Mutex::new(ErrorRepoter::default()));

        let lex = Lexer::new(file, text, encoding, errors.clone());
        Parser {
            errors,
            version,
            cur_token: None,
            lookahead_token: None,
            lex,
            require_user_variables: false,

            use_funcs: false,
            parsed_begin: false,
            got_statement: false,
            got_funcs: false,
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
                if let Some(lookahed) = self.lex.next_token() {
                    if lookahed == Token::If {
                        self.cur_token =
                            Some(Spanned::new(Token::ElseIf, start..self.lex.span().end));
                    } else {
                        self.lookahead_token =
                            Some(Spanned::new(lookahed, start..self.lex.span().end));
                    }
                }
            } else if is_case {
                let start = self.lex.span().start;
                if let Some(lookahed) = self.lex.next_token() {
                    if lookahed == Token::Else {
                        self.cur_token =
                            Some(Spanned::new(Token::Default, start..self.lex.span().end));
                    } else {
                        self.lookahead_token =
                            Some(Spanned::new(lookahed, start..self.lex.span().end));
                    }
                }
            } else if is_end {
                let start = self.lex.span().start;
                if let Some(lookahed) = self.lex.next_token() {
                    match lookahed {
                        Token::If => {
                            self.cur_token =
                                Some(Spanned::new(Token::EndIf, start..self.lex.span().end));
                        }
                        Token::While => {
                            self.cur_token =
                                Some(Spanned::new(Token::EndWhile, start..self.lex.span().end));
                        }
                        Token::Select => {
                            self.cur_token =
                                Some(Spanned::new(Token::EndSelect, start..self.lex.span().end));
                        }
                        Token::For => {
                            self.cur_token =
                                Some(Spanned::new(Token::Next, start..self.lex.span().end));
                        }
                        _ => {
                            let set_lookahad = if let Token::Identifier(id) = &lookahed {
                                if *id == *PROC_TOKEN {
                                    self.cur_token = Some(Spanned::new(
                                        Token::EndProc,
                                        start..self.lex.span().end,
                                    ));
                                    false
                                } else if *id == *FUNC_TOKEN {
                                    self.cur_token = Some(Spanned::new(
                                        Token::EndFunc,
                                        start..self.lex.span().end,
                                    ));
                                    false
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                            if set_lookahad {
                                self.lookahead_token =
                                    Some(Spanned::new(lookahed, start..self.lex.span().end));
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
        //println!("{}", std::backtrace::Backtrace::force_capture());
        self.errors.lock().unwrap().report_error(span, save_token);
        while self.get_cur_token().is_some()
            && self.get_cur_token() != Some(Token::Eol)
            && !matches!(self.get_cur_token(), Some(Token::Comment(_, _)))
        {
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
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(self.lex.span(), ParserWarningType::UsefuncsAlreadySet);
                }
                if self.got_statement {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(self.lex.span(), ParserErrorType::UsefuncAfterStatement);
                    self.next_token();
                    return None;
                }
                self.use_funcs = true;
                let cmt = self.save_spanned_token();
                self.next_token();
                return Some(AstNode::VariableDeclaration(Statement::Comment(
                    CommentAstNode::new(cmt),
                )));
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
                        self.report_error(
                            self.lex.span(),
                            ParserErrorType::NoStatementsAllowedOutsideBlock,
                        );
                        return None;
                    }
                    if self.got_funcs && !self.use_funcs {
                        self.report_error(
                            self.lex.span(),
                            ParserErrorType::NoStatementsAfterFunctions,
                        );
                        return None;
                    }
                    if !self.got_statement && !matches!(stmt, Statement::VariableDeclaration(_)) {
                        let mut main_block = vec![stmt];
                        loop {
                            let Some(cur_token) = &self.cur_token else {
                                break;
                            };
                            if cur_token.token == Token::Function
                                || cur_token.token == Token::Procedure
                            {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                main_block.push(stmt);
                            }
                        }
                        self.got_statement = true;
                        return Some(AstNode::Main(BlockStatement::empty(main_block)));
                    }
                    return Some(AstNode::VariableDeclaration(stmt));
                }
            }
        }
        None
    }
}

lazy_static::lazy_static! {
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

}

impl Parser {
    pub fn get_variable_type(&self) -> Option<VariableType> {
        if let Some(token) = &self.cur_token {
            if let Token::Identifier(id) = &token.token {
                if let Some(vt) = TYPE_HASHES.get(id) {
                    return Some(*vt);
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
    pub fn parse_var_info(&mut self) -> Option<VariableSpecifier> {
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::IdentifierExpected(self.save_token()),
            );
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        let mut dimensions = Vec::new();
        let mut leftpar_token = None;
        let mut rightpar_token = None;
        if let Some(Token::LPar) = &self.get_cur_token() {
            leftpar_token = Some(self.save_spanned_token());
            self.next_token();
            let Some(Token::Const(Constant::Integer(_))) = self.get_cur_token() else {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::NumberExpected(self.save_token()),
                );
                return None;
            };
            dimensions.push(DimensionSpecifier::new(self.save_spanned_token()));
            self.next_token();

            while let Some(Token::Comma) = &self.get_cur_token() {
                self.next_token();
                let Some(Token::Const(Constant::Integer(_))) = self.get_cur_token() else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::NumberExpected(self.save_token()),
                    );

                    return None;
                };
                dimensions.push(DimensionSpecifier::new(self.save_spanned_token()));
                self.next_token();
            }

            if dimensions.len() > 3 {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::TooManyDimensions(dimensions.len()),
                );

                return None;
            }

            if !matches!(self.get_cur_token(), Some(Token::RPar)) {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::MissingCloseParens(self.save_token()),
                );
                return None;
            }
            rightpar_token = Some(self.save_spanned_token());
            self.next_token();
        }
        Some(VariableSpecifier::new(
            identifier_token,
            leftpar_token,
            dimensions,
            rightpar_token,
        ))
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
            self.report_error(
                self.lex.span(),
                ParserErrorType::InvalidDeclaration(self.save_token()),
            );
            return None;
        };
        let func_or_proc_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(identifier)) = self.get_cur_token() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::IdentifierExpected(self.save_token()),
            );

            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(
                self.lex.span(),
                ParserErrorType::MissingOpenParens(self.save_token()),
            );
            return None;
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::MissingCloseParens(self.save_token()),
                );

                return None;
            }

            let mut var_token = None;
            if let Some(Token::Identifier(id)) = self.get_cur_token() {
                if id == Ascii::new("VAR".to_string()) {
                    if is_function {
                        self.report_error(
                            self.lex.span(),
                            ParserErrorType::VarNotAllowedInFunctions,
                        );
                    } else {
                        var_token = Some(self.save_spanned_token());
                    }
                    self.next_token();
                }
            }

            if let Some(var_type) = self.get_variable_type() {
                let type_token = self.save_spanned_token();
                self.next_token();
                let info = self.parse_var_info()?;
                parameters.push(ParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                ));
            } else {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::TypeExpected(self.save_token()),
                );
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
                self.report_error(
                    identifier_token.span,
                    ParserErrorType::StatementAlreadyDefined(self.save_token()),
                );
                return None;
            }

            return Some(AstNode::ProcedureDeclaration(
                ProcedureDeclarationAstNode::new(
                    declare_token,
                    func_or_proc_token,
                    identifier_token,
                    leftpar_token,
                    parameters,
                    rightpar_token,
                ),
            ));
        }
        if FunctionDefinition::get_function_definition(&identifier) >= 0 {
            self.report_error(
                identifier_token.span,
                ParserErrorType::FunctionAlreadyDefined(self.save_token()),
            );
            return None;
        }
        let Some(return_type) = self.get_variable_type() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::TypeExpected(self.save_token()),
            );
            return None;
        };
        let return_type_token = self.save_spanned_token();
        self.next_token();
        self.check_eol();
        Some(AstNode::FunctionDeclaration(
            FunctionDeclarationAstNode::new(
                declare_token,
                func_or_proc_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
                return_type_token,
                return_type,
            ),
        ))
    }

    fn check_eol(&mut self) -> bool {
        if self.get_cur_token() != Some(Token::Eol)
            && !matches!(self.get_cur_token(), Some(Token::Comment(_, _)))
        {
            let err_token = self.save_spanned_token();
            self.next_token();
            self.report_error(
                err_token.span,
                ParserErrorType::EolExpected(err_token.token),
            );
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
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::IdentifierExpected(self.save_token()),
                );

                return None;
            };
            let identifier_token = self.save_spanned_token();
            self.next_token();
            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::MissingOpenParens(self.save_token()),
                );
                return None;
            }

            let leftpar_token = self.save_spanned_token();
            self.next_token();

            let mut parameters = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                if self.get_cur_token().is_none() {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::MissingCloseParens(self.save_token()),
                    );

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

                    let info = self.parse_var_info()?;
                    parameters.push(ParameterSpecifier::new(
                        var_token, type_token, var_type, info,
                    ));
                } else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::TypeExpected(self.save_token()),
                    );
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

            while self.get_cur_token() != Some(Token::EndProc)
                && self.get_cur_token() != Some(Token::EndFunc)
            {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            let endproc_token = self.save_spanned_token();
            if endproc_token.token == Token::EndFunc {
                self.errors.lock().unwrap().report_warning(
                    endproc_token.span.clone(),
                    ParserWarningType::ProcedureClosedWithEndFunc,
                );
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
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::IdentifierExpected(self.save_token()),
                );

                return None;
            };
            let identifier_token = self.save_spanned_token();
            self.next_token();
            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::MissingOpenParens(self.save_token()),
                );
                return None;
            }

            let leftpar_token = self.save_spanned_token();
            self.next_token();

            let mut parameters = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                if self.get_cur_token().is_none() {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::MissingCloseParens(self.save_token()),
                    );

                    return None;
                }
                if let Some(Token::Identifier(id)) = self.get_cur_token() {
                    if id == Ascii::new("VAR".to_string()) {
                        self.report_error(
                            self.lex.span(),
                            ParserErrorType::VarNotAllowedInFunctions,
                        );
                        self.next_token();
                    }
                }

                if let Some(var_type) = self.get_variable_type() {
                    let type_token = self.save_spanned_token();
                    self.next_token();

                    let info = self.parse_var_info()?;
                    parameters.push(ParameterSpecifier::new(None, type_token, var_type, info));
                } else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::TypeExpected(self.save_token()),
                    );
                    return None;
                }

                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                }
            }
            let rightpar_token = self.save_spanned_token();
            self.next_token();

            let Some(return_type) = self.get_variable_type() else {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::TypeExpected(self.save_token()),
                );
                return None;
            };
            let return_type_token = self.save_spanned_token();
            self.next_token();
            self.skip_eol();

            let mut statements = Vec::new();

            while self.get_cur_token() != Some(Token::EndProc)
                && self.get_cur_token() != Some(Token::EndFunc)
            {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            let endfunc_token = self.save_spanned_token();
            if endfunc_token.token == Token::EndProc {
                self.errors.lock().unwrap().report_warning(
                    endfunc_token.span.clone(),
                    ParserWarningType::FunctionClosedWithEndProc,
                );
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
    input: &str,
    encoding: Encoding,
    version: u16,
) -> (Ast, Arc<Mutex<ErrorRepoter>>) {
    let mut nodes = Vec::new();
    let mut parser = Parser::new(file_name.clone(), input, encoding, version);
    parser.next_token();
    parser.skip_eol();

    while parser.cur_token.is_some() {
        if let Some(node) = parser.parse_ast_node() {
            nodes.push(node);
        }
    }

    (
        Ast {
            nodes,
            file_name,
            require_user_variables: parser.require_user_variables,
        },
        parser.errors.clone(),
    )
}

pub struct ErrorContainer {
    pub error: Box<dyn std::error::Error + Send + Sync>,
    pub span: core::ops::Range<usize>,
}

#[derive(Default)]
pub struct ErrorRepoter {
    pub errors: Vec<ErrorContainer>,
    pub warnings: Vec<ErrorContainer>,
}

impl ErrorRepoter {
    pub fn report_error<T: std::error::Error + 'static + Send + Sync>(
        &mut self,
        span: core::ops::Range<usize>,
        error: T,
    ) {
        self.errors.push(ErrorContainer {
            error: Box::new(error),
            span,
        });
    }

    pub fn report_warning<T: std::error::Error + 'static + Send + Sync>(
        &mut self,
        span: core::ops::Range<usize>,
        warning: T,
    ) {
        self.warnings.push(ErrorContainer {
            error: Box::new(warning),
            span,
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
    Utf8,
    CP437,
}

/// .
///
/// # Errors
///
/// This function will return an error if .
pub fn load_with_encoding(file_name: &Path, encoding: Encoding) -> std::io::Result<String> {
    let src_data = fs::read(file_name)?;
    let src = if encoding == Encoding::CP437 {
        let mut res = String::new();
        for b in src_data {
            res.push(CP437_TO_UNICODE[b as usize]);
        }
        res
    } else {
        String::from_utf8_lossy(&src_data).to_string()
    };
    Ok(src)
}
