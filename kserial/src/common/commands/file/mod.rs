mod open_file;
mod write_file;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;
pub use open_file::{FileResponse, OpenFile};
pub use write_file::{WriteFile, WriteFileResponse};

use crate::common::{
    fixed_null_str::{null_str, FixedNulString},
    validate::Validate,
};

#[derive(Debug, Pod, Zeroable, Clone, Copy, PartialEq, Eq, Validate)]
#[repr(transparent)]
pub struct FileHandle(i32);

impl FileHandle {
    pub(crate) const fn new(handle: i32) -> Self {
        Self(handle)
    }
    /// Is the file handle valid?
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
    /// Get the inner file handle
    pub fn inner(&self) -> i32 {
        self.0
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

impl Validate for FileFlags {
    fn validate(&self) -> bool {
        true
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
        const INVALID_HANDLE = 1 << 4;
    }
}

impl OsError {
    pub const fn is_ok(&self) -> bool {
        self.bits() == 0
    }
}

impl Validate for OsError {
    fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq, Validate)]
#[repr(C)]
pub struct IOError {
    pub err: OsError,
    pub err_str: null_str!(IOError::ERR_MAX_LEN),
}

impl IOError {
    pub const ERR_MAX_LEN: usize = 256;
    pub const INVALID_HANDLE: Self = Self::new(OsError::INVALID_HANDLE, "Invalid handle");
    pub const NOT_FOUND: Self = Self::new(OsError::NOT_FOUND, "File not found");
    pub const PERM_DENIED: Self = Self::new(OsError::PERM_DENIED, "Permission denied");
    pub const ALREADY_EXISTS: Self = Self::new(OsError::ALREADY_EXISTS, "File already exists");
    pub const OK: Self = Self::new(OsError::empty(), "");

    pub const fn new(err: OsError, err_str: &str) -> Self {
        let err_str = FixedNulString::from_str(err_str).unwrap();
        Self { err, err_str }
    }

    #[cfg(feature = "std")]
    pub fn from_io_err(err: std::io::Error) -> Self {
        use std::io::ErrorKind;

        use crate::common::commands::file::OsError;

        let str_err: null_str!(Self::ERR_MAX_LEN) =
            FixedNulString::from_str(&err.to_string()).unwrap();
        let err_code = match err.kind() {
            ErrorKind::NotFound => OsError::NOT_FOUND,
            ErrorKind::PermissionDenied => OsError::PERM_DENIED,
            ErrorKind::AlreadyExists => OsError::ALREADY_EXISTS,
            _ => OsError::UNKNOWN,
        };
        Self::new(err_code, &str_err)
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
