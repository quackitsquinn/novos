use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{
    array_vec::{varlen, ArrayVec},
    commands::ids::WRITE_FILE_ID,
    PacketContents,
};

use super::{FileHandle, IOError};

/// Command to write data to a file on the server.
#[derive(Debug, Clone, Copy, Zeroable, Pod, Validate)]
#[repr(C)]
pub struct WriteFile {
    file: FileHandle,
    _pad: [u8; WriteFile::PAD_LEN],
    data: varlen!(u8, WriteFile::CAPACITY),
}

impl WriteFile {
    const PAD_LEN: usize = 6;
    /// Maximum capacity of data to write in a single command.
    pub const CAPACITY: usize = 4096;

    /// Create a new `WriteFile` command.
    pub fn new(file: FileHandle, data: &[u8]) -> Option<Self> {
        let data = ArrayVec::try_from_bytes(data)?;
        if !file.is_valid() {
            return None;
        }
        Some(Self {
            file,
            _pad: [0; Self::PAD_LEN],
            data,
        })
    }

    /// Get the file handle.
    pub fn file(&self) -> FileHandle {
        self.file
    }

    /// Get the data to write.
    pub fn data(&self) -> &[u8] {
        &*self.data
    }
}

impl PacketContents for WriteFile {
    const ID: u8 = WRITE_FILE_ID;
}

/// Response from the server after writing to a file.
#[derive(Debug, Clone, Copy, Zeroable, Pod, Validate)]
#[repr(C)]
pub struct WriteFileResponse {
    /// Error code from the write operation.
    pub err: IOError,
}

impl WriteFileResponse {
    /// Create a new `WriteFileResponse`.
    pub fn ok() -> Self {
        Self::err(IOError::OK)
    }

    /// Create a new `WriteFileResponse` with the given error code.
    pub fn err(err: IOError) -> Self {
        Self { err }
    }

    /// Get the error code from the response.
    pub fn is_ok(&self) -> bool {
        self.err == IOError::OK
    }
}

impl PacketContents for WriteFileResponse {
    const ID: u8 = 0x02;
}
