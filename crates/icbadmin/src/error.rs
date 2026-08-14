use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    #[error("{0}")]
    Missing(String),

    #[error("Configuration could not be loaded: {0}")]
    Load(String),

    #[error("Configuration could not be saved: {0}")]
    Save(String),

    #[error("Invalid settings: {}", .0.join("; "))]
    Validation(Vec<String>),

    #[error("The configuration was modified by another tool since it was loaded. Reload and try again.")]
    Conflict,

    #[error("Another tool is currently writing to this board.")]
    Locked,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AdminError>;
