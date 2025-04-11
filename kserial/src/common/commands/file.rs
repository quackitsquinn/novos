use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

use crate::common::{
    fixed_null_str::{null_str, FixedNulString},
    PacketContents,
};

#[derive(Debug, Pod, Zeroable, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FileHandle(u64);

impl FileHandle {
    pub(crate) const fn new(handle: u64) -> Self {
        Self(handle)
    }
    /// Is the file handle valid?
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
#[repr(C)]
pub struct OpenFile {
    // This weird syntax is a const generic parameter. We don't use `Self` because it breaks `Pod` and `Zeroable`.
    pub filename: null_str!(OpenFile::FILENAME_MAX_LEN),
    pub flags: FileFlags,
}

impl OpenFile {
    pub const FILENAME_MAX_LEN: usize = 64;

    pub const fn new(filename: &str, flags: FileFlags) -> Option<Self> {
        let filename = FixedNulString::from_str(filename);
        if filename.is_none() {
            return None;
        }
        Some(Self {
            filename: filename.unwrap(),
            flags,
        })
    }

    pub const fn create(filename: &str) -> Option<Self> {
        Self::new(filename, FileFlags::CREATE_OVERWRITE)
    }

    pub const fn read(filename: &str) -> Option<Self> {
        Self::new(filename, FileFlags::READ)
    }

    pub const fn write(filename: &str) -> Option<Self> {
        Self::new(filename, FileFlags::WRITE)
    }
}

impl PacketContents for OpenFile {
    const ID: u8 = 0x01;
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
    #[repr(transparent)]
    pub struct FileFlags: u8 {
        const READ = 0b1;
        const WRITE = 0b1 << 1;
        const APPEND = 0b1 << 2;
        const CREATE = 0b1 << 3;
        // Convenience flags for const fns
        const CREATE_OVERWRITE = Self::WRITE.bits() | Self::CREATE.bits();
        const CREATE_APPEND = Self::APPEND.bits() | Self::CREATE.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct FileResponse {
    pub handle: FileHandle,
    pub err: null_str!(FileResponse::ERR_MAX_LEN),
}

impl PacketContents for FileResponse {
    const ID: u8 = OpenFile::ID;
}

impl FileResponse {
    pub const ERR_MAX_LEN: usize = 64;

    pub fn new(handle: u64) -> Self {
        Self {
            handle: FileHandle::new(handle),
            err: FixedNulString::new(),
        }
    }

    pub fn err(err: &str) -> Self {
        let mut response = Self::new(0);
        let err = FixedNulString::from_str(err).unwrap();
        response.err = err;
        response
    }
}
