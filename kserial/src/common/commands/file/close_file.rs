use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{commands::ids::CLOSE_FILE_ID, PacketContents};

use super::{FileHandle, IOError};

/// Command to close a file on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Validate)]
#[repr(transparent)]
pub struct CloseFile {
    /// The handle of the file to close.
    pub handle: FileHandle,
}

impl PacketContents for CloseFile {
    const ID: u8 = CLOSE_FILE_ID;
}

impl CloseFile {
    /// Create a new `CloseFile` command.
    pub fn new(handle: FileHandle) -> Self {
        Self { handle }
    }
}

/// Response from the server after closing a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Validate)]
#[repr(transparent)]
pub struct CloseFileResponse {
    err: IOError,
}

impl PacketContents for CloseFileResponse {
    const ID: u8 = CLOSE_FILE_ID;
}

impl CloseFileResponse {
    /// Create a new `CloseFileResponse`.
    pub(crate) fn new(err: IOError) -> Self {
        Self { err }
    }

    /// Get the error code from the response.
    pub fn err(&self) -> IOError {
        self.err
    }

    /// Returns true if the operation was successful.
    pub fn is_ok(&self) -> bool {
        self.err.err.is_ok()
    }
}
