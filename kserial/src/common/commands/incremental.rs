use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{
    array_vec::{varlen, ArrayVec},
    fixed_null_str::{null_str, FixedNulString},
    PacketContents,
};

use super::ids::{
    CLOSE_INCREMENTAL_FILE_CHANNEL_ID, CREATE_INCREMENTAL_FILE_CHANNEL_ID, INCREMENTAL_FILE_ID,
};

/// A command to create an incremental file channel.
///
/// An incremental file channel allows for sending regular updates of file data in chunks.
#[derive(Debug, Clone, Copy, Pod, Zeroable, Validate)]
#[repr(C)]
pub struct CreateIncrementalFileChannel {
    /// The name of the incremental file channel.
    pub name: null_str!(CreateIncrementalFileChannel::NAME_MAX_LEN),
    /// The file template for the incremental file channel.
    pub file_template: null_str!(CreateIncrementalFileChannel::FILE_TEMPLATE_MAX_LEN),
}

impl PacketContents for CreateIncrementalFileChannel {
    const ID: u8 = CREATE_INCREMENTAL_FILE_CHANNEL_ID;
}

impl CreateIncrementalFileChannel {
    /// The maximum length of the name string.
    pub const NAME_MAX_LEN: usize = 16;
    /// The maximum length of the file template string.
    pub const FILE_TEMPLATE_MAX_LEN: usize = 32;

    /// Create a new `CreateIncrementalFileChannel` command.
    pub fn new(name: &str, file_template: &str) -> Option<Self> {
        let name = FixedNulString::from_str(name)?;
        let file_template = FixedNulString::from_str(file_template)?;
        Some(Self {
            name,
            file_template,
        })
    }
}

/// A command to send a chunk of data for an incremental file.
#[derive(Debug, Clone, Copy, Pod, Zeroable, Validate)]
#[repr(C)]
pub struct IncrementalFile {
    /// The name of the incremental file.
    pub name: null_str!(IncrementalFile::NAME_MAX_LEN),
    /// Whether this is the final chunk of data for the file.
    pub is_done: u8,
    /// Half reserved for future use, half to remove padding bytes between `is_done` and `data`.
    _reserved: u8,
    /// The chunk of file data.
    pub data: varlen!(u8, IncrementalFile::MAX_DATA_SIZE),
}

impl IncrementalFile {
    /// The maximum length of the name string.
    pub const NAME_MAX_LEN: usize = 16; // The maximum length of the file name.
    /// The maximum size of the data field in bytes.
    pub const MAX_DATA_SIZE: usize = 4096; // The maximum size of the data field in bytes.

    /// Check if this is the final chunk of data for the file.
    pub fn is_done(&self) -> bool {
        self.is_done != 0
    }
}

impl PacketContents for IncrementalFile {
    const ID: u8 = INCREMENTAL_FILE_ID;
}

impl IncrementalFile {
    /// Create a new `IncrementalFile` command.
    pub fn new(name: &str, is_done: bool, data: &[u8]) -> Option<Self> {
        let name = FixedNulString::from_str(name)?;
        let data = ArrayVec::try_from_bytes(data)?;
        Some(Self {
            name,
            is_done: is_done as u8,
            _reserved: 0,
            data,
        })
    }
}

/// A command to close an incremental file channel.
#[derive(Debug, Clone, Copy, Pod, Zeroable, Validate)]
#[repr(C)]
pub struct CloseIncrementalFileChannel {
    /// The name of the incremental file channel to close.
    pub name: null_str!(CloseIncrementalFileChannel::NAME_MAX_LEN),
}

impl PacketContents for CloseIncrementalFileChannel {
    const ID: u8 = CLOSE_INCREMENTAL_FILE_CHANNEL_ID;
}

impl CloseIncrementalFileChannel {
    /// The maximum length of the name string.
    pub const NAME_MAX_LEN: usize = 16;
    /// Create a new `CloseIncrementalFileChannel` command.
    pub fn new(name: &str) -> Option<Self> {
        let name = FixedNulString::from_str(name)?;
        Some(Self { name })
    }
}
