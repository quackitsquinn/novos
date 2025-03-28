use bytemuck::{Pod, Zeroable};

use super::{array_vec::ArrayVec, fixed_null_str::FixedNulString, PacketContents};

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct StringPacket {
    // This weird syntax is a const generic parameter. We don't use `Self` because it breaks `Pod` and `Zeroable`.
    pub data: ArrayVec<u8, { StringPacket::CAPACITY }>,
}

impl PacketContents for StringPacket {
    const ID: u8 = 0x00;
}

impl StringPacket {
    pub const CAPACITY: usize = 128;

    pub fn new(data: &str) -> Option<Self> {
        let data = ArrayVec::from_str(data)?;
        Some(Self { data })
    }

    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        // Safety: The caller must ensure that the bytes are valid.
        // Also, this is a relatively no-op operation, so it's safe to mark as unsafe.
        let data = ArrayVec::from_bytes_unchecked(bytes);
        Self { data }
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CreateIncrementalFileChannel {
    pub name: FixedNulString<16>,
    pub file_template: FixedNulString<32>,
}

impl PacketContents for CreateIncrementalFileChannel {
    const ID: u8 = 0x06;
}

impl CreateIncrementalFileChannel {
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
    pub name: FixedNulString<16>,
    pub is_done: u8,
    /// Half reserved for future use, half to remove padding bytes between `is_done` and `data`.
    _reserved: u8,
    pub data: ArrayVec<u8, 4096>,
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
    pub name: FixedNulString<16>,
}

impl PacketContents for CloseIncrementalFileChannel {
    const ID: u8 = 0x08;
}

impl CloseIncrementalFileChannel {
    pub fn new(name: &str) -> Option<Self> {
        let name = FixedNulString::from_str(name)?;
        Some(Self { name })
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Shutdown {
    pub code: i32,
}

impl PacketContents for Shutdown {
    const ID: u8 = 0x09;
}

impl Shutdown {
    pub fn new(code: i32) -> Self {
        Self { code }
    }
}
