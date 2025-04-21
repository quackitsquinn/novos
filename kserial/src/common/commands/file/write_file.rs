use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{
    array_vec::{varlen, ArrayVec},
    PacketContents,
};

use super::{FileHandle, IOError};

#[derive(Debug, Clone, Copy, Zeroable, Pod, Validate)]
#[repr(C)]
pub struct WriteFile {
    file: FileHandle,
    _pad: [u8; WriteFile::PAD_LEN],
    data: varlen!(u8, WriteFile::CAPACITY),
}

impl WriteFile {
    const PAD_LEN: usize = 6;
    pub const CAPACITY: usize = 4096;

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

    pub fn file(&self) -> FileHandle {
        self.file
    }

    pub fn data(&self) -> &[u8] {
        &*self.data
    }
}

impl PacketContents for WriteFile {
    const ID: u8 = 0x02;
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, Validate)]
#[repr(C)]
pub struct WriteFileResponse {
    pub err: IOError,
}

impl WriteFileResponse {
    pub fn ok() -> Self {
        Self::err(IOError::OK)
    }

    pub fn err(err: IOError) -> Self {
        Self { err }
    }

    pub fn is_ok(&self) -> bool {
        self.err == IOError::OK
    }
}

impl PacketContents for WriteFileResponse {
    const ID: u8 = 0x02;
}
