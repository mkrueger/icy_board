use super::ZFrameType;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZModemError {
    #[error("Error invalid byte in subpacket got {0}/x{0:X} after ZDLE")]
    InvalidSubpacket(u8),
    #[error("invalid frame type {0}")]
    InvalidFrameType(u8),
    #[error("ZPAD expected got {0} (0x{0:X})")]
    ZPADExected(u8),
    #[error("ZDLE expected got {0} (0x{0:X})")]
    ZLDEExected(u8),
    #[error("unknown header type {0}")]
    UnknownHeaderType(u8),
    #[error("crc16 mismatch got 0x{0:04X} expected 0x{1:04X}")]
    CRC16Mismatch(u16, u16),
    #[error("crc32 mismatch got 0x{0:08X} expected 0x{1:08X}")]
    CRC32Mismatch(u32, u32),
    #[error("Got ZDATA before ZFILE")]
    ZDataBeforeZFILE,
    #[error("unsupported frame {0:?}")]
    UnsupportedFrame(ZFrameType),
    #[error("hex number expected")]
    HexNumberExpected,
    #[error("{0}")]
    GenericError(String),
    #[error("Error during subpacket crc check: {0}")]
    SubpacketCrcError(String),
    #[error("no file open")]
    NoFileOpen,
}
