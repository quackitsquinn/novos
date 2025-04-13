use bytemuck::Zeroable;

use crate::common::{
    commands::{FileFlags, FileHandle, OpenFile},
    PacketContents,
};

use super::{cfg::is_packet_mode, send_string, serial::SerialClient, SerialAdapter};

pub struct File {
    handle: FileHandle,
    open_mode: FileFlags,
}
// TODO: Destructor when `CloseFile` is implemented

impl File {
    /// Create a new file object.
    /// # Safety
    /// The caller must ensure that both the handle and open mode are valid.
    pub unsafe fn new(handle: FileHandle, open_mode: FileFlags) -> Self {
        Self { handle, open_mode }
    }

    pub fn create_file_with(serial: &SerialClient, name: &str) -> Option<Self> {
        let open_file = OpenFile::create(name)?;
        // Copy the flags so that if the flags for `create` are changed, we don't have to change this function.
        let flags = open_file.flags;

        if !is_packet_mode() {
            send_string("Packet mode is not enabled, cannot create file.");
            return None;
        }

        unsafe { serial.send_pod(&open_file.into_packet()) };
        let mut response: FileHandle = FileHandle::zeroed();
        unsafe { serial.read_pod(&mut response) };

        if response.is_valid() {
            Some(unsafe { Self::new(response, flags) })
        } else {
            None
        }
    }

    pub fn create_file(name: &str) -> Option<Self> {
        Self::create_file_with(&super::SERIAL_ADAPTER, name)
    }

    pub fn handle(&self) -> &FileHandle {
        &self.handle
    }

    pub fn open_mode(&self) -> &FileFlags {
        &self.open_mode
    }
}
