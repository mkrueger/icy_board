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
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::unnecessary_wraps,
    clippy::trivially_copy_pass_by_ref,
    clippy::used_underscore_binding,
    clippy::unused_self,
    clippy::ref_option,
    clippy::large_enum_variant,
    clippy::struct_field_names
)]

pub mod ast;
#[path = "ast/color.rs"]
pub mod color;
pub mod compiler;
pub mod crypt;
pub mod datetime;
pub mod decompiler;
pub mod executable;
pub mod formatting;
pub mod hir;
pub mod io;
pub mod parser;
pub mod password;
pub mod search_patterns;
pub mod semantic;
pub mod tables;
pub mod tokens;

pub type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
