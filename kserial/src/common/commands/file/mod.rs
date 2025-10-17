mod close_file;
mod open_file;
mod write_file;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
pub use close_file::{CloseFile, CloseFileResponse};
use kserial_derive::Validate;
pub use open_file::{FileResponse, OpenFile};
pub use write_file::{WriteFile, WriteFileResponse};

use crate::common::{
    fixed_null_str::{null_str, FixedNulString},
    validate::Validate,
};

/// A handle to a file on the server.
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
    /// Flags for opening a file.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
    #[repr(transparent)]
    pub struct FileFlags: u8 {
        /// Read the file
        const READ = 0b1;
        /// Write to the file
        const WRITE = 0b1 << 1;
        /// Append to the file
        const APPEND = 0b1 << 2;
        /// Create the file if it does not exist
        const CREATE = 0b1 << 3;
        // Convenience flags for const fns
        /// Create the file if it does not exist, and overwrite it if it does
        const CREATE_OVERWRITE = Self::WRITE.bits() | Self::CREATE.bits();
        /// Create the file if it does not exist, and append to it if it does
        const CREATE_APPEND = Self::APPEND.bits() | Self::CREATE.bits();
    }
}

impl Validate for FileFlags {
    fn validate(&self) -> bool {
        true
    }
}

bitflags! {
    /// An error from the server's OS.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
    #[repr(transparent)]
    pub struct OsError: u32 {
        /// No error
        const OK = 0;
        /// The file was not found
        const NOT_FOUND = 1 << 0;
        /// Permission denied
        const PERM_DENIED = 1 << 1;
        /// The file already exists
        const ALREADY_EXISTS = 1 << 2;
        /// An unknown error occurred
        const UNKNOWN = 1 << 3;
        /// An invalid handle was used
        const INVALID_HANDLE = 1 << 4;
    }
}

impl OsError {
    /// Returns true if the error is OK
    pub const fn is_ok(&self) -> bool {
        self.bits() == Self::OK.bits()
    }
}

impl Validate for OsError {
    fn validate(&self) -> bool {
        true
    }
}

/// An IO Error from the server's OS.
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq, Validate)]
#[repr(C)]
pub struct IOError {
    /// The type of the error
    pub err: OsError,
    /// The string representation of the error. This will contain the error of `err` is UNKNOWN
    pub err_str: null_str!(IOError::ERR_MAX_LEN),
}

impl IOError {
    /// The max length for the string representation of the error
    pub const ERR_MAX_LEN: usize = 256;
    /// Error for an invalid handle
    pub const INVALID_HANDLE: Self = Self::new(OsError::INVALID_HANDLE, "Invalid handle");
    /// Error for a file that cannot be found
    pub const NOT_FOUND: Self = Self::new(OsError::NOT_FOUND, "File not found");
    /// Error for permission denied
    pub const PERM_DENIED: Self = Self::new(OsError::PERM_DENIED, "Permission denied");
    /// Error for a file that already exists
    pub const ALREADY_EXISTS: Self = Self::new(OsError::ALREADY_EXISTS, "File already exists");
    /// No error
    pub const OK: Self = Self::new(OsError::empty(), "");

    /// Create a new IOError
    pub const fn new(err: OsError, err_str: &str) -> Self {
        let err_str = FixedNulString::from_str(err_str).unwrap();
        Self { err, err_str }
    }

    /// Converts the given std::io::Error into an IOError
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

    /// Create an empty IOError
    pub const fn empty() -> Self {
        Self {
            err: OsError::empty(),
            err_str: FixedNulString::from_str("").unwrap(),
        }
    }

    /// Is the error OK?
    pub fn is_ok(&self) -> bool {
        self.err.is_ok()
    }
}
