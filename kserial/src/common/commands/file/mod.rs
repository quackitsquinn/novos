mod open_file;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
pub use open_file::{FileResponse, OpenFile};

use crate::common::fixed_null_str::{null_str, FixedNulString};

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

bitflags::bitflags! {
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

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
    #[repr(transparent)]
    pub struct OsError: u32 {
        const NOT_FOUND = 1 << 0;
        const PERM_DENIED = 1 << 1;
        const ALREADY_EXISTS = 1 << 2;
        const UNKNOWN = 1 << 3;
    }
}

impl OsError {
    pub const fn is_ok(&self) -> bool {
        self.bits() == 0
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
#[repr(C)]
pub struct IOError {
    pub err: OsError,
    pub err_str: null_str!(FileResponse::ERR_MAX_LEN),
}

impl IOError {
    pub fn new(err: OsError, err_str: &str) -> Self {
        let err_str = FixedNulString::from_str(err_str).unwrap();
        Self { err, err_str }
    }

    pub fn empty() -> Self {
        Self {
            err: OsError::empty(),
            err_str: FixedNulString::from_str("").unwrap(),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.err.is_ok()
    }
}
