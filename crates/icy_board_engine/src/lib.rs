#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::struct_excessive_bools,
    clippy::module_name_repetitions
)]

use std::{env, error::Error, path::PathBuf};

use semver::Version;

pub mod icy_board;
pub mod vm;

pub mod ast;
pub mod compiler;
pub mod crypt;
pub mod datetime;
pub mod decompiler;
pub mod executable;
pub mod formatting;
pub mod parser;
pub mod search_patterns;
pub mod semantic;
pub mod tables;
pub mod tokens;

pub type Res<T> = Result<T, Box<dyn Error + Send + Sync>>;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

pub const DEFAULT_ICYBOARD_FILE: &str = "icboard.toml";

pub enum IcyBoardFileLookupError {
    FileNotFound(PathBuf),
}

pub fn lookup_icyboard_file(file: &Option<PathBuf>) -> Option<PathBuf> {
    resolve_icyboard_file(file).ok()
}

pub fn resolve_icyboard_file(file: &Option<PathBuf>) -> Result<PathBuf, IcyBoardFileLookupError> {
    resolve_icyboard_file_with_env(file, env::var_os("ICB_PATH").map(PathBuf::from))
}

fn resolve_icyboard_file_with_env(file: &Option<PathBuf>, icb_path: Option<PathBuf>) -> Result<PathBuf, IcyBoardFileLookupError> {
    let explicit_path = file.is_some();
    let mut file_path = file.clone().unwrap_or(PathBuf::from("."));
    if file_path.is_dir() {
        file_path = file_path.join(DEFAULT_ICYBOARD_FILE);
    }

    let file_path = file_path.with_extension("toml");
    if file_path.exists() {
        return Ok(file_path);
    }
    if !explicit_path && let Some(mut path) = icb_path {
        if path.is_dir() {
            path.push(DEFAULT_ICYBOARD_FILE);
        }
        if path.exists() {
            return Ok(path);
        }
    }

    Err(IcyBoardFileLookupError::FileNotFound(file_path))
}

#[cfg(test)]
mod tests {
    use super::{IcyBoardFileLookupError, resolve_icyboard_file_with_env};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn explicit_missing_path_does_not_fall_back_to_icb_path() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("icboard.toml"), "").unwrap();
        let missing = directory.path().join("missing.toml");

        let result = resolve_icyboard_file_with_env(&Some(missing.clone()), Some(directory.path().to_path_buf()));

        assert!(matches!(result, Err(IcyBoardFileLookupError::FileNotFound(path)) if path == missing));
    }

    #[test]
    fn omitted_path_can_fall_back_to_icb_path() {
        let directory = tempdir().unwrap();
        let board = directory.path().join("icboard.toml");
        fs::write(&board, "").unwrap();

        let result = resolve_icyboard_file_with_env(&None, Some(directory.path().to_path_buf()));

        assert_eq!(result.ok(), Some(board));
    }
}
