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

use std::error::Error;

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

/*
#[cfg(test)]
pub mod tests;
*/
pub type Res<T> = Result<T, Box<dyn Error + Send + Sync>>;
