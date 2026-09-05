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

    #[error("expected block {0} but got {1}")]
    OutOfSyncBlock(u8, u8),

    #[error("file is incomplete: expected {0} bytes but received {1}")]
    IncompleteFile(u64, u64),

    #[error("no file open")]
    NoFileOpen,

    #[error("no block available for retransmission")]
    NoPendingBlock,

    #[error("invalid response received: {0}")]
    InvalidResponse(u8),

    #[error("timeout waiting for data")]
    Timeout,
}
