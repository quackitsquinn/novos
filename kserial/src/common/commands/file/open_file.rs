use bytemuck::{Pod, Zeroable};

use crate::common::{
    commands::FileHandle,
    fixed_null_str::{null_str, FixedNulString},
    PacketContents,
};

use super::{FileFlags, IOError, OsError};

#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
#[repr(C)]
pub struct OpenFile {
    // This weird syntax is a const generic parameter. We don't use `Self` because it breaks `Pod` and `Zeroable`.
    pub path: null_str!(OpenFile::FILENAME_MAX_LEN),
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
            path: filename.unwrap(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct FileResponse {
    pub handle: FileHandle,
    _pad: [u8; 4],
    pub err: IOError,
}

impl PacketContents for FileResponse {
    const ID: u8 = OpenFile::ID;
}

impl FileResponse {
    pub const ERR_MAX_LEN: usize = 64;

    pub fn new(handle: i32) -> Self {
        assert!(
            handle != 0,
            "File handle must be non-zero! Use `err` instead."
        );
        Self {
            handle: FileHandle::new(handle),
            _pad: [0; 4],
            err: IOError::empty(),
        }
    }

    pub fn err(code: OsError, err: &str) -> Self {
        let mut response = Self::new(0);
        response.err = IOError::new(code, err);
        response
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
        Self::err(err_code, &str_err)
    }
}
