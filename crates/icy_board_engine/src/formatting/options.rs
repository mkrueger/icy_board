/*
pub enum BinopSeparator {
    Front,
    Back
}*/

use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
#[serde_inline_default]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormattingOptions {
    #[serde_inline_default(true)]
    pub space_around_binop: bool,

    //pub binop_separator: BinopSeparator,
    #[serde_inline_default(false)]
    pub use_tabs: bool,

    #[serde_inline_default(4)]
    pub indent_size: usize,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self::DEFAULT.clone()
    }
}

impl FormattingOptions {
    pub const DEFAULT: Self = Self {
        space_around_binop: true,
        use_tabs: false,
        indent_size: 4,
    };

    pub fn new() -> Self {
        Self::default()
    }
}
