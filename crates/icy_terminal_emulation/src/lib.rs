//! The terminal emulations known to icy_net and icy_engine.
//!
//! This crate only holds the type, so that crates which need it but not the networking of
//! icy_net (for example icy_engine on wasm32) can use it without pulling in tokio or rustls.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TerminalEmulation {
    #[default]
    Ansi,
    Utf8Ansi,
    Avatar,
    Ascii,
    PETscii,
    ATAscii,
    ViewData,
    Mode7,
    Rip,
    Skypix,
    AtariST,
}
