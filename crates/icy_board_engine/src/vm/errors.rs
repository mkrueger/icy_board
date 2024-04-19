use crate::executable::VariableType;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IcyError {
    #[error("Parameter {0} should be from type String")]
    ParameterStringExpected(u32),

    #[error("Parameter {0} should be from type Integer")]
    ParameterIntegerExpected(u32),

    #[error("File channel should be 0 <= 7 it was: {0}")]
    FileChannelOutOfBounds(i32),

    #[error("{0} should be from type Integer")]
    IntegerExpected(String),

    #[error("not supported")]
    NotSupported,

    #[error("Variable {0} not found.")]
    VariableNotFound(String),

    #[error("User {0} not found.")]
    UserNotFound(String),

    #[error("User not set.")]
    UserNotSet,

    #[error("Sort dest array should be int, was {0}.")]
    SortDestinationArrayIntRequired(VariableType),

    #[error("loading {0} file ({1}): {2}")]
    ErrorLoadingFile(String, String, String),

    #[error("{0} file not found ({1})")]
    FileNotFound(String, String),

    #[error("Invalid MNU file ({0}) : {1}")]
    InvalidMNU(String, String),

    #[error("Error generating TOML ({0}) : {1}")]
    ErrorGeneratingToml(String, String),

    #[error("Error saving file ({0}) : {1}")]
    ErrorSavingFile(String, String),
}
