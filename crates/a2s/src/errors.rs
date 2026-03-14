use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to reserve memory: {0}")]
    TryReserveError(#[from] std::collections::TryReserveError),

    #[error("unexpected response header: expected 0x{expected:02X}, got 0x{actual:02X}")]
    UnexpectedHeader { expected: u8, actual: u8 },

    #[error("packet too short: expected at least {expected} bytes, got {actual}")]
    PacketTooShort { expected: usize, actual: usize },

    #[error("multi-packet response exceeds limits")]
    MultiPacketTooLarge,

    #[error("multi-packet fragment ID mismatch")]
    MismatchPacketId,

    #[error("invalid bz2 decompressed size")]
    InvalidBz2Size,

    #[error("decompressed checksum does not match")]
    ChecksumMismatch,

    #[error("no binary chunks found in rules")]
    NoBinaryChunks,

    #[error("steam ID length exceeds 8 bytes")]
    SteamIdTooLong,

    #[error("expected boolean (0 or 1) for {field}, got 0x{value:02X}")]
    InvalidBool { field: &'static str, value: u8 },
}
