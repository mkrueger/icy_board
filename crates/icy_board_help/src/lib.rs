pub mod catalog;
pub mod document;
pub mod pcboard;
pub mod theme;

pub use document::{Document, Role, StyledLine, StyledSpan};
pub use pcboard::{Encoding, RenderOptions, RenderedHelp, render};
pub use theme::HelpTheme;

use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn invalid(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
}
