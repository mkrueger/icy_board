pub mod connection;
pub use connection::*;
pub mod binkp;
pub mod crc;
pub mod iemsi;
pub mod pattern_recognizer;
pub mod protocol;
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

    #[error("Binkp frame carries {0} octets, the size field holds 32767")]
    BinkpFrameTooLarge(usize),

    #[error("Unknown binkp command id {0}")]
    UnknownBinkpCommand(u8),

    #[error("Binkp session refused by the remote: {0}")]
    BinkpRemoteError(String),

    #[error("Binkp remote is busy: {0}")]
    BinkpRemoteBusy(String),

    #[error("Called {0} but reached {1}")]
    BinkpWrongSystem(String, String),

    #[error("Binkp sent a {0} frame during session setup")]
    BinkpUnexpectedFrame(String),

    #[error("{0}: cannot parse args")]
    BinkpBadArgument(String, String),

    #[error("Binkp session timed out")]
    BinkpTimeout,
}
