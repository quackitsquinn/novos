#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("Invalid command")]
    InvalidCommand,
    #[error("Invalid checksum")]
    InvalidChecksum,
    #[error("Unexpected io error {0}")]
    IOError(#[from] std::io::Error),
}

impl PacketError {
    pub fn is_invalid_checksum(&self) -> bool {
        matches!(self, PacketError::InvalidChecksum)
    }

    pub fn is_invalid_command(&self) -> bool {
        matches!(self, PacketError::InvalidCommand)
    }

    pub fn is_io_error(&self) -> bool {
        matches!(self, PacketError::IOError(_))
    }

    pub fn io_error(&self) -> Option<&std::io::Error> {
        if let PacketError::IOError(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
