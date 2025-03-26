use bytemuck::{Pod, Zeroable};

use super::{array_vec::ArrayVec, fixed_null_str::FixedNulString, PacketContents};

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct SendString {
    pub data: ArrayVec<u8, 4096>,
}

impl PacketContents for SendString {
    const ID: u8 = 0x00;
}

impl SendString {
    pub fn new(data: &str) -> Option<Self> {
        let data = ArrayVec::from_str(data)?;
        Some(Self { data })
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IncrementalFile {
    pub name: FixedNulString<16>,
    pub is_done: bool,
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
            is_done,
            data,
        })
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
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
