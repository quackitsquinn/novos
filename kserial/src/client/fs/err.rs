//! File system related errors.
use crate::common::commands::IOError;

/// File errors that can occur.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FileError {
    /// Not in packet mode
    #[error("Not in packet mode")]
    NotInPacketMode,
    /// Filename too long
    #[error("Filename too long")]
    FilenameTooLong,
    /// Read error
    #[error("Read error")]
    ReadError,
    /// Generic operating system error
    #[error("Operating System Error: {0:?}")]
    IoError(IOError),
}
