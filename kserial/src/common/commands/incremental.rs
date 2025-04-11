use bytemuck::{Pod, Zeroable};

use crate::common::{
    array_vec::{varlen, ArrayVec},
    fixed_null_str::{null_str, FixedNulString},
    PacketContents,
};

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CreateIncrementalFileChannel {
    pub name: null_str!(CreateIncrementalFileChannel::NAME_MAX_LEN),
    pub file_template: null_str!(CreateIncrementalFileChannel::FILE_TEMPLATE_MAX_LEN),
}

impl PacketContents for CreateIncrementalFileChannel {
    const ID: u8 = 0x06;
}

impl CreateIncrementalFileChannel {
    pub const NAME_MAX_LEN: usize = 16;
    pub const FILE_TEMPLATE_MAX_LEN: usize = 32;

    pub fn new(name: &str, file_template: &str) -> Option<Self> {
        let name = FixedNulString::from_str(name)?;
        let file_template = FixedNulString::from_str(file_template)?;
        Some(Self {
            name,
            file_template,
        })
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct IncrementalFile {
    pub name: null_str!(IncrementalFile::NAME_MAX_LEN),
    pub is_done: u8,
    /// Half reserved for future use, half to remove padding bytes between `is_done` and `data`.
    _reserved: u8,
    pub data: varlen!(u8, IncrementalFile::MAX_DATA_SIZE),
}

impl IncrementalFile {
    pub const NAME_MAX_LEN: usize = 16; // The maximum length of the file name.
    pub const MAX_DATA_SIZE: usize = 4096; // The maximum size of the data field in bytes.

    pub fn is_done(&self) -> bool {
        self.is_done != 0
    }
}

impl PacketContents for IncrementalFile {
    const ID: u8 = 0x07;
}

impl IncrementalFile {
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

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CloseIncrementalFileChannel {
    pub name: null_str!(CloseIncrementalFileChannel::NAME_MAX_LEN),
}

impl PacketContents for CloseIncrementalFileChannel {
    const ID: u8 = 0x08;
}

impl CloseIncrementalFileChannel {
    const NAME_MAX_LEN: usize = 16;
    pub fn new(name: &str) -> Option<Self> {
        let name = FixedNulString::from_str(name)?;
        Some(Self { name })
    }
}
