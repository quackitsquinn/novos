use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{
    commands::{ids::OPEN_FILE_ID, FileHandle},
    fixed_null_str::{null_str, FixedNulString},
    PacketContents,
};

use super::{FileFlags, IOError, OsError};

/// Command to open a file on the server.
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq, Validate)]
#[repr(C)]
pub struct OpenFile {
    /// The path to the file to open.
    pub path: null_str!(OpenFile::FILENAME_MAX_LEN),
    /// The flags to open the file with.
    pub flags: FileFlags,
}

impl OpenFile {
    /// Maximum length of a filename (including null terminator).
    pub const FILENAME_MAX_LEN: usize = 64;

    /// Create a new `OpenFile` command.
    /// Returns `None` if the filename is too long.
    pub const fn new(filename: &str, flags: FileFlags) -> Option<Self> {
        let filename = FixedNulString::from_str(filename);
        if filename.is_none() {
            return None;
        }
        Some(Self {
            path: filename.unwrap(),
            flags,
        })
    }

    /// Create a new `OpenFile` command to create a file, overwriting it if it already exists.
    /// Returns `None` if the filename is too long.
    pub const fn create(filename: &str) -> Option<Self> {
        Self::new(filename, FileFlags::CREATE_OVERWRITE)
    }

    /// Create a new `OpenFile` command to open a file for reading.
    /// Returns `None` if the filename is too long.
    pub const fn read(filename: &str) -> Option<Self> {
        Self::new(filename, FileFlags::READ)
    }

    /// Create a new `OpenFile` command to open a file for writing.
    /// Returns `None` if the filename is too long.
    /// Note that this does not create the file if it does not exist. Use `create` for that.
    pub const fn write(filename: &str) -> Option<Self> {
        Self::new(filename, FileFlags::WRITE)
    }
}

impl PacketContents for OpenFile {
    const ID: u8 = OPEN_FILE_ID;
}

/// Response from the server after opening a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Validate)]
#[repr(C)]
pub struct FileResponse {
    /// The handle of the opened file. Will be 0 if there was an error.
    pub handle: FileHandle,
    /// Padding.
    _zero: [u8; 4],
    /// An error code, if any.
    pub err: IOError,
}

impl PacketContents for FileResponse {
    const ID: u8 = OpenFile::ID;
}

impl FileResponse {
    /// Maximum length of an error string (including null terminator).
    pub const ERR_MAX_LEN: usize = 64;

    /// Create a new `FileResponse`.
    pub fn new(handle: i32) -> Self {
        assert!(
            handle != 0,
            "File handle must be non-zero! Use `err` instead."
        );
        Self {
            handle: FileHandle::new(handle),
            _zero: [0; 4],
            err: IOError::empty(),
        }
    }

    /// Create a new `FileResponse` with an error.
    pub fn err(code: OsError, err: &str) -> Self {
        let mut response = Self::new(0);
        response.err = IOError::new(code, err);
        response
    }

    /// Create a new `FileResponse` from a `std::io::Error`.
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
        Self::err(err_code, &str_err)
    }
}
