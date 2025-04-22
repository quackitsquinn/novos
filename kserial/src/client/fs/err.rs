use crate::common::commands::IOError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum FileError {
    #[error("Not in packet mode")]
    NotInPacketMode,
    #[error("Filename too long")]
    FilenameTooLong,
    #[error("Read error")]
    ReadError,
    #[error("Operating System Error: {0:?}")]
    IoError(IOError),
}
