use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use crate::{
    ast::{
        Ast, AstNode, BlockStatement, CommentAstNode, Constant, DimensionSpecifier, EnumDeclarationAstNode, EnumVariantSpecifier, FunctionDeclarationAstNode,
        FunctionImplementation, FunctionParameterSpecifier, ParameterSpecifier, ProcedureDeclarationAstNode, ProcedureImplementation,
        ProcedureParameterSpecifier, Statement, TypeDeclarationAstNode, TypeFieldSpecifier, VariableParameterSpecifier, VariableSpecifier, const_value,
    },
    compiler::{
        user_data::{UserData, UserDataRegistry},
        workspace::Workspace,
    },
    executable::{FuncOpCode, FunctionDefinition, OpCode, StatementDefinition, VariableType},
    icy_board::{conferences::Conference, doors::Door, file_directory::FileDirectory, message_area::MessageArea},
};

use self::lexer::{Lexer, Spanned, Token};
use codepages::tables::CP437_TO_UNICODE;
use thiserror::Error;
use unicase::Ascii;

mod expression;
pub mod lexer;
pub mod pre_processor_expr_visitor;
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

    #[error("'END' expected before the end of the file")]
    BlockEndExpected,

    #[error("'END' only closes a BEGIN...END block, use 'EXIT' to end a program or 'STOP' to abort one")]
    EndIsNotAStatement,

    #[error("A program can only have one BEGIN...END block")]
    BlockAlreadyDefined,

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

    #[error("Type '{0}' is already declared")]
    TypeAlreadyDeclared(unicase::Ascii<String>),

    #[error("Field '{0}' is already declared in this type")]
    FieldAlreadyDeclared(unicase::Ascii<String>),

    #[error("'ENDTYPE' expected before the end of the file")]
    EndTypeExpected,

    #[error("'ENDENUM' expected before the end of the file")]
    EndEnumExpected,

    #[error("An enum needs at least one member")]
    EnumNeedsAMember,

    #[error("Enum member '{0}' is already declared")]
    EnumMemberAlreadyDeclared(unicase::Ascii<String>),

    #[error("An enum member needs an integer value the compiler can work out")]
    EnumValueExpected,

    #[error("A type needs at least one field")]
    TypeNeedsAField,

    #[error("A type can't hold a field of its own type ('{0}')")]
    TypeUsedInItself(unicase::Ascii<String>),

    #[error("Record field '{0}' cannot be an array")]
    TypeFieldArrayNotSupported(unicase::Ascii<String>),

    #[error("Record field '{0}' cannot have an initializer")]
    TypeFieldInitializerNotSupported(unicase::Ascii<String>),

    #[error("Board object {0} cannot be a record field")]
    TypeFieldBoardObjectNotSupported(VariableType),

    #[error("No room for another type, {0} is the most a program may declare")]
    TooManyTypes(usize),

    #[error("No room for another field, {0} is the most a type may hold")]
    TooManyFields(usize),

    #[error("'TYPE' needs runtime {0}, an older PPE has nowhere to store the layout")]
    TypeNeedsNewerRuntime(u16),
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

/// A record a program declared with `TYPE ... ENDTYPE`.
#[derive(Debug, Clone, PartialEq)]
pub struct UserTypeDefinition {
    pub id: usize,
    pub name: unicase::Ascii<String>,
    pub fields: Vec<(unicase::Ascii<String>, VariableType)>,
}

/// An integer-backed type that exists in source only.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDefinition {
    pub id: u8,
    pub name: unicase::Ascii<String>,
    pub variants: Vec<(unicase::Ascii<String>, i32)>,
}

impl EnumDefinition {
    pub fn value(&self, variant: &unicase::Ascii<String>) -> Option<i32> {
        self.variants.iter().find_map(|(name, value)| (name == variant).then_some(*value))
    }

    /// A member standing for `value`, so a lowered value can be written as itself again.
    pub fn variant_name(&self, value: i32) -> Option<&unicase::Ascii<String>> {
        self.variants.iter().find_map(|(name, v)| (*v == value).then_some(name))
    }
}

impl UserTypeDefinition {
    pub fn field_index(&self, name: &unicase::Ascii<String>) -> Option<usize> {
        self.fields.iter().position(|(field, _)| field == name)
    }

    pub fn field_type(&self, index: usize) -> Option<VariableType> {
        self.fields.get(index).map(|(_, t)| *t)
    }
}

#[derive(Default)]
pub struct UserTypeRegistry {
    pub registered_types: HashMap<unicase::Ascii<String>, VariableType>,
    pub types: HashMap<u8, UserDataRegistry>,
    built_in_records: HashMap<u8, UserTypeDefinition>,
    /// Records the compiled program declares. Shared across every file of a
    /// compilation so a type declared in one is visible in the next.
    user_types: RwLock<Vec<UserTypeDefinition>>,
    enums: RwLock<Vec<EnumDefinition>>,
}

pub const FIRST_ID: usize = 30;
pub const CONFERENCE_ID: usize = 30;
pub const MESSAGE_AREA_ID: usize = 31;
pub const FILE_DIRECTORY_ID: usize = 32;
pub const DOOR_ID: usize = 33;
pub const CONTACT_ID: usize = 34;
pub const SURFACE_ID: usize = 35;
pub const EVENT_ID: usize = 36;
pub const AUDIO_ID: usize = 37;
pub const TERM_STATE_ID: usize = 38;
pub const ERROR_ID: usize = 39;
pub const TERM_INFO_ID: usize = 40;

/// The board objects are ours, so no `PCBoard` language knows their names.
pub const FIRST_BOARD_OBJECT_LANGUAGE_VERSION: u16 = 400;

/// Types a program declares itself start here, so the board can keep adding
/// objects of its own below without ever running into them.
pub const FIRST_USER_TYPE_ID: usize = 100;

/// How many records one program may declare, ids 100..=255.
pub const MAX_USER_TYPES: usize = u8::MAX as usize - FIRST_USER_TYPE_ID + 1;

/// How many fields one record may hold - the PPE stores the count in a byte.
pub const MAX_TYPE_FIELDS: usize = u8::MAX as usize;

/// True for a type a program declared rather than one the board provides.
pub fn is_user_declared_type(id: u8) -> bool {
    id as usize >= FIRST_USER_TYPE_ID
}

impl UserTypeRegistry {
    pub fn icy_board_registry() -> Self {
        let mut reg = UserTypeRegistry::default();
        reg.register::<Conference>(CONFERENCE_ID);
        reg.register::<MessageArea>(MESSAGE_AREA_ID);
        reg.register::<FileDirectory>(FILE_DIRECTORY_ID);
        reg.register::<Door>(DOOR_ID);
        reg.register_record(
            CONTACT_ID,
            "CONTACT",
            vec![
                (unicase::Ascii::new("Service".to_string()), VariableType::String),
                (unicase::Ascii::new("Account".to_string()), VariableType::String),
            ],
        );
        reg.register::<crate::icy_board::state::ppl_surface::PplSurface>(SURFACE_ID);
        reg.register::<crate::icy_board::state::ppl_events::PplEvent>(EVENT_ID);
        reg.register::<crate::icy_board::state::ppl_audio::PplAudio>(AUDIO_ID);
        reg.register::<crate::icy_board::state::ppl_terminal_state::PplTerminalState>(TERM_STATE_ID);
        reg.register::<crate::icy_board::state::ppl_error::PplError>(ERROR_ID);
        reg.register::<crate::icy_board::state::ppl_terminal_info::PplTerminalInfo>(TERM_INFO_ID);

        reg
    }

    pub fn get_type(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        self.get_board_object(identifier).or_else(|| self.get_declared_type(identifier))
    }

    /// One of the objects the board itself provides, such as a conference.
    pub fn get_board_object(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        self.registered_types.get(identifier).copied()
    }

    /// A record or an enum the program declared for itself.
    pub fn get_declared_type(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        self.get_user_type(identifier)
            .map(|def| VariableType::UserData(def.id as u8))
            .or_else(|| self.get_enum(identifier).map(|def| VariableType::UserData(def.id)))
    }

    /// The record declared under `identifier`, if the program declared one.
    pub fn get_user_type(&self, identifier: &unicase::Ascii<String>) -> Option<UserTypeDefinition> {
        self.user_types.read().unwrap().iter().find(|def| def.name == *identifier).cloned()
    }

    pub fn get_user_type_from_id(&self, id: u8) -> Option<UserTypeDefinition> {
        let id = id as usize;
        if id < FIRST_USER_TYPE_ID {
            return None;
        }
        self.user_types.read().unwrap().get(id - FIRST_USER_TYPE_ID).cloned()
    }

    pub fn get_record_type_from_id(&self, id: u8) -> Option<UserTypeDefinition> {
        self.built_in_records.get(&id).cloned().or_else(|| self.get_user_type_from_id(id))
    }

    pub fn is_record_type(&self, id: u8) -> bool {
        self.built_in_records.contains_key(&id) || is_user_declared_type(id)
    }

    pub fn user_types(&self) -> Vec<UserTypeDefinition> {
        self.user_types.read().unwrap().clone()
    }

    pub fn get_enum(&self, identifier: &unicase::Ascii<String>) -> Option<EnumDefinition> {
        self.enums.read().unwrap().iter().find(|def| def.name == *identifier).cloned()
    }

    pub fn enums(&self) -> Vec<EnumDefinition> {
        self.enums.read().unwrap().clone()
    }

    pub fn get_enum_from_id(&self, id: u8) -> Option<EnumDefinition> {
        self.enums.read().unwrap().iter().find(|def| def.id == id).cloned()
    }

    pub fn is_enum_type(&self, variable_type: VariableType) -> bool {
        matches!(variable_type, VariableType::UserData(id) if self.get_enum_from_id(id).is_some())
    }

    /// Records grow upward from 100, enums downward from 255; neither kind is
    /// serialized under the other's representation.
    pub fn declare_enum(&self, name: unicase::Ascii<String>, variants: Vec<(unicase::Ascii<String>, i32)>) -> Option<u8> {
        let mut enums = self.enums.write().unwrap();
        let id = u8::MAX as usize - enums.len();
        let next_record = FIRST_USER_TYPE_ID + self.user_types.read().unwrap().len();
        if id < next_record {
            return None;
        }
        enums.push(EnumDefinition { id: id as u8, name, variants });
        Some(id as u8)
    }

    /// The position of a field inside a record, which doubles
    /// as its member id in the generated code.
    pub fn record_field_index(&self, id: u8, field: &unicase::Ascii<String>) -> Option<usize> {
        self.get_record_type_from_id(id)?.field_index(field)
    }

    /// Adds a record and hands back its type id, or `None` when the id space is full.
    pub fn declare_user_type(&self, name: unicase::Ascii<String>, fields: Vec<(unicase::Ascii<String>, VariableType)>) -> Option<usize> {
        let mut user_types = self.user_types.write().unwrap();
        let id = FIRST_USER_TYPE_ID + user_types.len();
        let lowest_enum = self.enums.read().unwrap().last().map_or(u8::MAX as usize + 1, |def| def.id as usize);
        if id >= lowest_enum {
            return None;
        }
        user_types.push(UserTypeDefinition { id, name, fields });
        Some(id)
    }

    /// Every PPE stores the type id, so an id that is taken can never move.
    fn claim_id(&self, id: usize, name: &str) {
        assert!(
            (FIRST_ID..FIRST_USER_TYPE_ID).contains(&id),
            "board object '{name}' wants id {id}, which is outside the board object range"
        );
        assert!(
            !self.types.contains_key(&(id as u8)) && !self.built_in_records.contains_key(&(id as u8)),
            "board object '{name}' wants id {id}, which is already taken"
        );
    }

    pub fn register<T: UserData>(&mut self, id: usize) {
        self.claim_id(id, T::TYPE_NAME);
        let mut registry = UserDataRegistry::default();
        T::register_members(&mut registry);
        self.registered_types
            .insert(unicase::Ascii::new(T::TYPE_NAME.to_string()), VariableType::UserData(id as u8));
        self.types.insert(id as u8, registry);
    }

    fn register_record(&mut self, id: usize, name: &str, fields: Vec<(unicase::Ascii<String>, VariableType)>) {
        self.claim_id(id, name);
        self.registered_types
            .insert(unicase::Ascii::new(name.to_string()), VariableType::UserData(id as u8));
        self.built_in_records.insert(
            id as u8,
            UserTypeDefinition {
                id,
                name: unicase::Ascii::new(name.to_string()),
                fields,
            },
        );
    }

    pub fn get_type_from_id(&self, d: u8) -> Option<&UserDataRegistry> {
        self.types.get(&d)
    }
}

pub struct Parser<'a> {
    pub error_reporter: Arc<Mutex<ErrorReporter>>,

    pub type_registry: &'a UserTypeRegistry,
    lang_version: u16,
    pub require_user_variables: bool,

    cur_token: Option<Spanned<Token>>,
    lookahead_token: Option<Spanned<Token>>,
    lex: Lexer,

    // parser state
    use_funcs: bool,
    parsed_begin: bool,
    parsed_block: bool,
    got_statement: bool,
    got_funcs: bool,
    in_function: bool,
    types_predeclared: bool,
}
static PROC_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PROC".to_string()));
static FUNC_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("FUNC".to_string()));
static ON_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("ON".to_string()));
static ERROR_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("ERROR".to_string()));

impl<'a> Parser<'a> {
    pub fn new(
        file: PathBuf,
        error_reporter: Arc<Mutex<ErrorReporter>>,
        type_registry: &'a UserTypeRegistry,
        text: &str,
        encoding: Encoding,
        workspace: &Workspace,
    ) -> Self {
        let lex: Lexer = Lexer::new(file, workspace, text, encoding, error_reporter.clone());
        let lang_version = lex.lang_version();
        Parser {
            error_reporter,
            lang_version,
            cur_token: None,
            lookahead_token: None,
            lex,
            require_user_variables: false,
            type_registry,
            use_funcs: false,
            parsed_begin: false,
            parsed_block: false,
            got_statement: false,
            got_funcs: false,
            in_function: false,
            types_predeclared: false,
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
            // Neither word is reserved on its own, so `ON` stays usable as a name.
            let is_on = self.lang_version >= 400 && token == Token::Identifier(ON_TOKEN.clone());
            self.cur_token = Some(Spanned::new(token, self.lex.span()));

            if is_on {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahed) = self.lex.next_token() {
                    if lookahed == Token::Identifier(ERROR_TOKEN.clone()) {
                        self.cur_token = Some(Spanned::new(Token::OnError, start..self.lex.span().end));
                    } else {
                        self.lookahead_token = Some(Spanned::new(lookahed, end..self.lex.span().end));
                    }
                }
            } else if is_else {
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
                        Token::Type => {
                            self.cur_token = Some(Spanned::new(Token::EndType, start..self.lex.span().end));
                        }
                        Token::Enum => {
                            self.cur_token = Some(Spanned::new(Token::EndEnum, start..self.lex.span().end));
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
        if let Some(token) = &self.cur_token { token.span.clone() } else { 0..0 }
    }

    fn save_token(&self) -> Token {
        if let Some(token) = &self.cur_token { token.token.clone() } else { Token::Eol }
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
            Token::Type => {
                let original_reporter = if self.types_predeclared {
                    Some(std::mem::replace(&mut self.error_reporter, Arc::new(Mutex::new(ErrorReporter::default()))))
                } else {
                    None
                };
                let declaration = self.parse_type_declaration();
                if let Some(original_reporter) = original_reporter {
                    self.error_reporter = original_reporter;
                }
                if let Some(decl) = declaration {
                    return Some(AstNode::TypeDeclaration(decl));
                }
            }
            Token::Enum => {
                let original_reporter = if self.types_predeclared {
                    Some(std::mem::replace(&mut self.error_reporter, Arc::new(Mutex::new(ErrorReporter::default()))))
                } else {
                    None
                };
                let declaration = self.parse_enum_declaration();
                if let Some(original_reporter) = original_reporter {
                    self.error_reporter = original_reporter;
                }
                if let Some(decl) = declaration {
                    return Some(AstNode::EnumDeclaration(decl));
                }
            }
            Token::Begin => {
                if self.parsed_block {
                    self.report_error(cur_token.span.clone(), ParserErrorType::BlockAlreadyDefined);
                    return None;
                }
                let (begin_token, statements, end_token) = self.parse_block_body()?;
                self.parsed_block = true;
                self.got_statement = true;
                return Some(AstNode::Main(BlockStatement::new(begin_token, statements, end_token)));
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
                    if let Statement::Label(label) = &stmt
                        && *label.get_label() == *statements::BEGIN_LABEL
                    {
                        self.parsed_begin = true;
                    }

                    if self.parsed_block || (self.use_funcs && !self.parsed_begin) {
                        if matches!(stmt, Statement::Comment(_) | Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_)) {
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
                    if !self.got_statement && !matches!(stmt, Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_) | Statement::Comment(_)) {
                        let mut main_block = vec![stmt];
                        while let Some(cur_token) = &self.cur_token {
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

    /// Parses `TYPE <name> ... ENDTYPE` and registers the record so later
    /// declarations can name it as a type.
    fn parse_type_declaration(&mut self) -> Option<TypeDeclarationAstNode> {
        let type_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(name)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        let type_already_declared = self.type_registry.get_type(&name).is_some() || built_in_type(&name, self.lang_version).is_some();
        if !self.types_predeclared && type_already_declared {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TypeAlreadyDeclared(name.clone()));
        }

        let mut fields: Vec<TypeFieldSpecifier> = Vec::new();
        let mut field_names: Vec<Ascii<String>> = Vec::new();

        let endtype_token = loop {
            while matches!(self.get_cur_token(), Some(Token::Eol)) || matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
                self.next_token();
            }
            match self.get_cur_token() {
                Some(Token::EndType) => {
                    let token = self.save_spanned_token();
                    self.next_token();
                    break token;
                }
                None => {
                    self.report_error(self.lex.span(), ParserErrorType::EndTypeExpected);
                    return None;
                }
                _ => {}
            }

            // A record can't contain itself, that has no finite layout.
            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == name
            {
                self.report_error(self.save_token_span(), ParserErrorType::TypeUsedInItself(name.clone()));
                continue;
            }

            let Some(field_type) = self.get_variable_type() else {
                self.report_error(self.save_token_span(), ParserErrorType::InvalidToken(self.save_token()));
                continue;
            };
            let field_type_token = self.save_spanned_token();
            if !self.types_predeclared && matches!(field_type, VariableType::UserData(id) if !is_user_declared_type(id)) {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_error(field_type_token.span.clone(), ParserErrorType::TypeFieldBoardObjectNotSupported(field_type));
            }
            self.next_token();

            while let Some(specifier) = self.parse_var_info(false) {
                let field_name = specifier.get_identifier().clone();
                if !specifier.get_dimensions().is_empty() {
                    self.error_reporter.lock().unwrap().report_error(
                        specifier.get_identifier_token().span.clone(),
                        ParserErrorType::TypeFieldArrayNotSupported(field_name.clone()),
                    );
                }
                if specifier.get_initalizer().is_some() {
                    self.error_reporter.lock().unwrap().report_error(
                        specifier.get_identifier_token().span.clone(),
                        ParserErrorType::TypeFieldInitializerNotSupported(field_name.clone()),
                    );
                }
                if field_names.contains(&field_name) {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(specifier.get_identifier_token().span.clone(), ParserErrorType::FieldAlreadyDeclared(field_name));
                } else {
                    field_names.push(field_name);
                }
                fields.push(TypeFieldSpecifier::new(field_type_token.clone(), field_type, specifier));

                if matches!(self.get_cur_token(), Some(Token::Comma)) {
                    self.next_token();
                    continue;
                }
                break;
            }
        };

        if fields.is_empty() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TypeNeedsAField);
            return None;
        }

        if fields.len() > MAX_TYPE_FIELDS {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TooManyFields(MAX_TYPE_FIELDS));
            return None;
        }

        let field_layout = fields.iter().map(|field| (field.get_identifier().clone(), field.get_variable_type())).collect();
        if !self.types_predeclared && !type_already_declared && self.type_registry.declare_user_type(name, field_layout).is_none() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TooManyTypes(MAX_USER_TYPES));
            return None;
        }

        Some(TypeDeclarationAstNode::new(type_token, identifier_token, fields, endtype_token))
    }

    fn parse_enum_declaration(&mut self) -> Option<EnumDeclarationAstNode> {
        let enum_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(name)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        let type_already_declared = self.type_registry.get_type(&name).is_some() || built_in_type(&name, self.lang_version).is_some();
        if !self.types_predeclared && type_already_declared {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TypeAlreadyDeclared(name.clone()));
        }

        let mut variants = Vec::new();
        let mut names = Vec::new();
        let mut next_value = 0i32;
        let endenum_token = loop {
            while matches!(self.get_cur_token(), Some(Token::Eol | Token::Comma)) || matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
                self.next_token();
            }
            match self.get_cur_token() {
                Some(Token::EndEnum) => {
                    let token = self.save_spanned_token();
                    self.next_token();
                    break token;
                }
                None => {
                    self.report_error(self.lex.span(), ParserErrorType::EndEnumExpected);
                    return None;
                }
                _ => {}
            }

            let Some(Token::Identifier(variant_name)) = self.get_cur_token() else {
                self.report_error(self.save_token_span(), ParserErrorType::IdentifierExpected(self.save_token()));
                continue;
            };
            let variant_token = self.save_spanned_token();
            self.next_token();

            if names.contains(&variant_name) {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_error(variant_token.span.clone(), ParserErrorType::EnumMemberAlreadyDeclared(variant_name.clone()));
            } else {
                names.push(variant_name);
            }

            let (eq_token, explicit_value, value) = if self.get_cur_token() == Some(Token::Eq) {
                let eq = self.save_spanned_token();
                self.next_token();
                let Some(expr) = self.parse_expression() else {
                    self.report_error(self.save_token_span(), ParserErrorType::EnumValueExpected);
                    continue;
                };
                let Some(value) = const_value(&expr, &|_| None).map(|value| value.as_int()) else {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(expr.get_span(), ParserErrorType::EnumValueExpected);
                    continue;
                };
                (Some(eq), Some(expr), value)
            } else {
                (None, None, next_value)
            };
            next_value = value.saturating_add(1);
            variants.push(EnumVariantSpecifier::new(variant_token, eq_token, value, explicit_value));
        };

        if variants.is_empty() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::EnumNeedsAMember);
            return None;
        }

        let layout = variants.iter().map(|variant| (variant.get_identifier().clone(), variant.get_value())).collect();
        if !self.types_predeclared && !type_already_declared && self.type_registry.declare_enum(name, layout).is_none() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TooManyTypes(MAX_USER_TYPES));
            return None;
        }

        Some(EnumDeclarationAstNode::new(enum_token, identifier_token, variants, endenum_token))
    }

    fn parse_function_parameter_specifier(&mut self) -> ParameterSpecifier {
        let func_token = self.save_spanned_token();
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters: Vec<ParameterSpecifier> = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
            }

            if self.lang_version >= 350 {
                if let Some(Token::Function) = self.get_cur_token() {
                    parameters.push(self.parse_function_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }

                if let Some(Token::Procedure) = self.get_cur_token() {
                    parameters.push(self.parse_procedure_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
            }

            let mut var_token = None;
            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == Ascii::new("VAR".to_string())
            {
                var_token = Some(self.save_spanned_token());
                self.next_token();
            }
            if let Some(var_type) = self.get_variable_type() {
                let type_token = self.save_spanned_token();
                self.next_token();
                let info = self.parse_var_info(false);
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                )));
            } else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
            }

            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
            }
        }
        let rightpar_token = self.save_spanned_token();
        let return_type_token = self.next_token();

        let Some(return_type) = self.get_variable_type() else {
            self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
        };
        self.next_token();

        ParameterSpecifier::Function(FunctionParameterSpecifier::new(
            func_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            return_type_token.unwrap(),
            return_type,
        ))
    }
    fn parse_procedure_parameter_specifier(&mut self) -> ParameterSpecifier {
        let proc_token = self.save_spanned_token();
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters: Vec<ParameterSpecifier> = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
            }

            if self.lang_version >= 350 {
                if let Some(Token::Function) = self.get_cur_token() {
                    parameters.push(self.parse_function_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }

                if let Some(Token::Procedure) = self.get_cur_token() {
                    parameters.push(self.parse_procedure_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
            }

            let mut var_token = None;
            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == Ascii::new("VAR".to_string())
            {
                var_token = Some(self.save_spanned_token());
                self.next_token();
            }
            if let Some(var_type) = self.get_variable_type() {
                let type_token = self.save_spanned_token();
                self.next_token();
                let info = self.parse_var_info(false);
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                )));
            } else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
            }

            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
            }
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();

        ParameterSpecifier::Procedure(ProcedureParameterSpecifier::new(
            proc_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
        ))
    }
}

static BUILT_IN_TYPE_LOOKUP: std::sync::LazyLock<HashMap<unicase::Ascii<String>, (VariableType, u16)>> = std::sync::LazyLock::new(|| {
    let mut m = HashMap::new();
    for (name, variable_type, since) in BUILT_IN_TYPES {
        m.insert(unicase::Ascii::new((*name).to_string()), (*variable_type, *since));
    }
    m
});

/// Which language version gave each built-in type its name, from the PPL release
/// notes: 2.00 brought the numeric widths and the big string, 3.00 the `DBase` date.
static BUILT_IN_TYPES: &[(&str, VariableType, u16)] = &[
    ("INTEGER", VariableType::Integer, 100),
    ("STRING", VariableType::String, 100),
    ("BOOLEAN", VariableType::Boolean, 100),
    ("DATE", VariableType::Date, 100),
    ("TIME", VariableType::Time, 100),
    ("MONEY", VariableType::Money, 100),
    ("SDWORD", VariableType::Integer, 200),
    ("LONG", VariableType::Integer, 200),
    ("BIGSTR", VariableType::BigStr, 200),
    ("EDATE", VariableType::EDate, 200),
    ("WORD", VariableType::Word, 200),
    ("UWORD", VariableType::Word, 200),
    ("SWORD", VariableType::SWord, 200),
    ("INT", VariableType::SWord, 200),
    ("BYTE", VariableType::Byte, 200),
    ("UBYTE", VariableType::Byte, 200),
    ("UNSIGNED", VariableType::Unsigned, 200),
    ("DWORD", VariableType::Unsigned, 200),
    ("UDWORD", VariableType::Unsigned, 200),
    ("SBYTE", VariableType::SByte, 200),
    ("SHORT", VariableType::SByte, 200),
    ("REAL", VariableType::Float, 200),
    ("FLOAT", VariableType::Float, 200),
    ("DOUBLE", VariableType::Double, 200),
    ("DREAL", VariableType::Double, 200),
    ("DDATE", VariableType::DDate, 300),
    ("MSGAREAID", VariableType::MessageAreaID, 400),
];

/// The built-in type that name stands for, or nothing if the language did not have
/// it yet.
pub fn built_in_type(name: &unicase::Ascii<String>, lang_version: u16) -> Option<VariableType> {
    BUILT_IN_TYPE_LOOKUP
        .get(name)
        .filter(|(_, since)| *since <= lang_version)
        .map(|(variable_type, _)| *variable_type)
}

/// The type names a program written for that language version may use.
pub fn built_in_type_names(lang_version: u16) -> Vec<&'static str> {
    BUILT_IN_TYPES
        .iter()
        .filter(|(_, _, since)| *since <= lang_version)
        .map(|(name, _, _)| *name)
        .collect()
}

impl Parser<'_> {
    pub fn get_variable_type(&self) -> Option<VariableType> {
        if let Some(token) = &self.cur_token {
            if let Token::Identifier(id) = &token.token {
                if let Some(vt) = built_in_type(id, self.lang_version) {
                    return Some(vt);
                }
                if self.lang_version >= FIRST_BOARD_OBJECT_LANGUAGE_VERSION
                    && let Some(vt) = self.type_registry.get_board_object(id)
                {
                    return Some(vt);
                }
                // An enum is a type from 350 on, so this is not gated with the objects.
                if let Some(vt) = self.type_registry.get_declared_type(id) {
                    return Some(vt);
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
        if can_be_empty && (matches!(self.get_cur_token(), Some(Token::Comma)) || matches!(self.get_cur_token(), Some(Token::RPar))) {
            return None;
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
        } else if self.lang_version >= 350
            && let Some(Token::Eq) = self.get_cur_token()
        {
            let eq_token = self.save_spanned_token();
            self.next_token();
            let initializer = self.parse_expression();
            return Some(VariableSpecifier::new(identifier_token, None, dimensions, None, Some(eq_token), initializer));
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

            if self.lang_version >= 350 {
                if let Some(Token::Function) = self.get_cur_token() {
                    parameters.push(self.parse_function_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
                if let Some(Token::Procedure) = self.get_cur_token() {
                    parameters.push(self.parse_procedure_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
            }

            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == Ascii::new("VAR".to_string())
            {
                if is_function {
                    self.report_error(self.lex.span(), ParserErrorType::VarNotAllowedInFunctions);
                } else {
                    var_token = Some(self.save_spanned_token());
                }
                self.next_token();
            }
            if let Some(var_type) = self.get_variable_type() {
                let type_token = self.save_spanned_token();
                self.next_token();
                let info = self.parse_var_info(true);
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                )));
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
                if self.lang_version >= 350 {
                    if let Some(Token::Function) = self.get_cur_token() {
                        parameters.push(self.parse_function_parameter_specifier());
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                        }
                        continue;
                    }
                    if let Some(Token::Procedure) = self.get_cur_token() {
                        parameters.push(self.parse_procedure_parameter_specifier());
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                        }
                        continue;
                    }
                }

                let mut var_token = None;
                if let Some(Token::Identifier(id)) = self.get_cur_token()
                    && id.eq_ignore_ascii_case("VAR")
                {
                    var_token = Some(self.save_spanned_token());
                    self.next_token();
                }

                if let Some(var_type) = self.get_variable_type() {
                    let type_token = self.save_spanned_token();
                    self.next_token();

                    let info = self.parse_var_info(false);
                    parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                        var_token, type_token, var_type, info,
                    )));
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
                if let Some(Token::Identifier(id)) = self.get_cur_token()
                    && id == Ascii::new("VAR".to_string())
                {
                    self.report_error(self.lex.span(), ParserErrorType::VarNotAllowedInFunctions);
                    self.next_token();
                }

                if let Some(var_type) = self.get_variable_type() {
                    let type_token = self.save_spanned_token();
                    self.next_token();

                    let info = self.parse_var_info(false);
                    parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, type_token, var_type, info)));
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
    workspace: &Workspace,
) -> Ast {
    parse_ast_internal(file_name, error_reporter, input, user_types, encoding, workspace, false)
}

pub fn parse_ast_with_predeclared_types(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
) -> Ast {
    parse_ast_internal(file_name, error_reporter, input, user_types, encoding, workspace, true)
}

fn parse_ast_internal(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
    types_predeclared: bool,
) -> Ast {
    error_reporter.lock().unwrap().set_file_name(&file_name);
    let mut nodes = Vec::new();
    let mut parser = Parser::new(file_name.clone(), error_reporter, user_types, input, encoding, workspace);
    parser.types_predeclared = types_predeclared;
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
        language_version: parser.lang_version,
        require_user_variables: parser.require_user_variables,
    }
}

pub fn preparse_type_declarations(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
) {
    error_reporter.lock().unwrap().set_file_name(&file_name);
    // The whole file is read again for the real parse, which reports everything it
    // finds. Only what the declarations themselves say is new here.
    let scratch = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut parser = Parser::new(file_name, scratch, user_types, input, encoding, workspace);
    parser.next_token();
    while parser.cur_token.is_some() {
        if matches!(parser.get_cur_token(), Some(Token::Type | Token::Enum)) {
            let scratch = std::mem::replace(&mut parser.error_reporter, error_reporter.clone());
            if parser.get_cur_token() == Some(Token::Type) {
                parser.parse_type_declaration();
            } else {
                parser.parse_enum_declaration();
            }
            parser.error_reporter = scratch;
        } else {
            parser.next_token();
        }
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

    pub fn report_error_file<T: std::error::Error + 'static + Send + Sync>(&mut self, file_name: PathBuf, span: core::ops::Range<usize>, error: T) {
        self.errors.push(ErrorContainer {
            error: Box::new(error),
            span,
            file_name,
        });
    }

    pub fn report_warning<T: std::error::Error + 'static + Send + Sync>(&mut self, span: core::ops::Range<usize>, warning: T) {
        self.warnings.push(ErrorContainer {
            error: Box::new(warning),
            span,
            file_name: self.cur_file.clone(),
        });
    }

    pub fn report_warning_file<T: std::error::Error + 'static + Send + Sync>(&mut self, file_name: PathBuf, span: core::ops::Range<usize>, warning: T) {
        self.warnings.push(ErrorContainer {
            error: Box::new(warning),
            span,
            file_name,
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
