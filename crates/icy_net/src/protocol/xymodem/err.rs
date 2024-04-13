use thiserror::Error;

#[derive(Error, Debug)]
pub enum XYModemError {
    #[error("transmission canceled")]
    Cancel,

    #[error("invalid x/y modem mode: {0}")]
    InvalidMode(u8),

    #[error("too many retries sending ymodem header")]
    TooManyRetriesSendingHeader,

    #[error("only 1 file can be send with x-modem")]
    XModem1File,

    #[error("too many retries starting the communication")]
    TooManyRetriesStarting,

    #[error("too many retries reading block")]
    TooManyRetriesReadingBlock,

    #[error("no file open")]
    NoFileOpen,
}
