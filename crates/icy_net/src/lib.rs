pub mod connection;
pub use connection::*;
pub mod binkp;
pub mod crc;
pub mod iemsi;
pub mod pattern_recognizer;
pub mod protocol;
pub mod terminal;
pub mod zconnect;

use semver::Version;
use thiserror::Error;
pub mod termcap_detect;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}
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

    #[error("Operation is unsupported")]
    Unsupported,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid EMSI packet")]
    InvalidEmsiPacket,

    #[error("Invalid CRC32 in EMSI")]
    EmsiCRC32Error,
}
