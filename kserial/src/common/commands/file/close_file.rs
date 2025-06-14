use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{commands::ids::CLOSE_FILE_ID, PacketContents};

use super::{FileHandle, IOError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Validate)]
#[repr(transparent)]
pub struct CloseFile {
    pub handle: FileHandle,
}

impl PacketContents for CloseFile {
    const ID: u8 = CLOSE_FILE_ID;
}

impl CloseFile {
    pub fn new(handle: FileHandle) -> Self {
        Self { handle }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Validate)]
#[repr(transparent)]
pub struct CloseFileResponse {
    err: IOError,
}

impl PacketContents for CloseFileResponse {
    const ID: u8 = CLOSE_FILE_ID;
}

impl CloseFileResponse {
    pub fn new(err: IOError) -> Self {
        Self { err }
    }

    pub fn err(&self) -> IOError {
        self.err
    }

    pub fn is_ok(&self) -> bool {
        self.err.err.is_ok()
    }
}
