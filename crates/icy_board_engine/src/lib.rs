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
pub mod parser;
pub mod semantic;
pub mod tables;
pub mod tokens;

pub type Res<T> = Result<T, Box<dyn Error + Send + Sync>>;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

pub const DEFAULT_ICYBOARD_FILE: &str = "icboard.toml";

pub fn lookup_icyboard_file(file: &Option<PathBuf>) -> Option<PathBuf> {
    let mut file = file.clone().unwrap_or(PathBuf::from("."));
    if file.is_dir() {
        file = file.join(DEFAULT_ICYBOARD_FILE);
    }

    let file = file.with_extension("toml");
    if file.exists() {
        return Some(file);
    }

    if let Ok(var) = env::var("ICB_PATH") {
        let mut path = PathBuf::from(var);
        path.push(DEFAULT_ICYBOARD_FILE);
        if path.exists() {
            return Some(path);
        }
    }

    None
}
