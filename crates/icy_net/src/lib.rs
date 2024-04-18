pub mod connection;
pub use connection::*;
pub mod crc;
pub mod iemsi;
pub mod protocol;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("Could not connect to any address")]
    CouldNotConnect,

    #[error("Maximum Emsi ICI header size exceeded ({0})")]
    MaximumEmsiICIExceeded(usize),

    #[error("Invalid escape sequence in EMSI")]
    InvalidEscapeInEmsi,

    #[error("Invalid Unicode in EMSI")]
    NoUnicodeInEmsi,
}