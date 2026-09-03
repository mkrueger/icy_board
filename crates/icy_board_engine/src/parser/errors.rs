use crate::executable::{FuncOpCode, OpCode, VariableType};

use super::lexer::Token;
use thiserror::Error;

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

    #[error("Expected 'IN' ({0})")]
    InExpected(Token),

    #[error("Expected expression ({0})")]
    ExpressionExpected(Token),

    #[error("Expression nesting exceeds the supported limit of {0}")]
    ExpressionNestingTooDeep(usize),

    #[error("Statement nesting exceeds the supported limit of {0}")]
    StatementNestingTooDeep(usize),

    #[error("Expected statement")]
    StatementExpected,

    #[error("Too many dimensions for variable '{0}' (max 3)")]
    TooManyDimensions(usize),

    #[error("Invalid token '{0}' - 'CASE' expected")]
    CaseExpected(Token),

    #[error("Unexpected identifier ({0})")]
    UnknownIdentifier(String),

    #[error("'{0}' is a built-in constant and can't be assigned to")]
    ConstantIsNotAssignable(Token),

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

    #[error("'ENDMODULE' expected before the end of the file")]
    EndModuleExpected,

    #[error("MODULE expects a name")]
    ModuleNameExpected,

    #[error("Only one MODULE is allowed in a source file")]
    ModuleAlreadyDefined,

    #[error("{0} is only valid directly inside a MODULE")]
    VisibilityOutsideModule(String),

    #[error("IMPORT expects 'AS' and a local alias")]
    InvalidImport,

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

    #[error("Record field '{0}' has a dimension above 65535")]
    TypeFieldDimensionTooLarge(unicase::Ascii<String>),

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
    #[error("PPL 4.00 array declarations should use '[' and ']' instead of '(' and ')'")]
    ArrayBracketsRequired,

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
