use bytemuck::Zeroable;

use core::fmt::Write;

use crate::common::{
    commands::{FileFlags, FileHandle, FileResponse, OpenFile, WriteFile, WriteFileResponse},
    packet::Packet,
    PacketContents,
};

use super::{cfg::is_packet_mode, send_string, serial::SerialClient, SerialWriter};

pub struct File<'a> {
    handle: FileHandle,
    open_mode: FileFlags,
    client: &'a SerialClient,
}
// TODO: Destructor when `CloseFile` is implemented

impl<'a> File<'a> {
    /// Create a new file object.
    /// # Safety
    /// The caller must ensure that both the handle and open mode are valid.
    pub unsafe fn new(handle: FileHandle, open_mode: FileFlags, client: &'a SerialClient) -> Self {
        Self {
            handle,
            open_mode,
            client,
        }
    }

    pub fn create_file_with(serial: &'a SerialClient, name: &str) -> Option<Self> {
        let open_file = OpenFile::create(name)?;
        // Copy the flags so that if the flags for `create` are changed, we don't have to change this function.
        let flags = open_file.flags;

        // TODO: Allow these values to be overridden by the inner SerialAdapter type
        if !is_packet_mode() {
            // TODO: User configurable error handling / Error type
            //send_string("Packet mode is not enabled, cannot create file.");
            return None;
        }

        serial.send_packet(&open_file.into_packet());
        let response: Packet<FileResponse> = serial.read_packet()?;
        let response = response.payload();

        if !response.err.is_ok() {
            return None;
        }

        let handle = response.handle;

        let file = unsafe { File::new(handle, flags, serial) };
        Some(file)
    }

    pub fn create_file(name: &str) -> Option<Self> {
        Self::create_file_with(&super::SERIAL_ADAPTER, name)
    }

    pub fn write(&self, data: &[u8]) -> Option<()> {
        if !is_packet_mode() {
            send_string("Packet mode is not enabled, cannot write to file.");
            return None;
        }

        for chunk in data.chunks(WriteFile::CAPACITY) {
            let write_file = WriteFile::new(self.handle, chunk).expect("infallible");
            let packet = write_file.into_packet();
            // writeln!(SerialWriter, "{:?}", packet).ok();
            // writeln!(SerialWriter, "{:?}", bytemuck::bytes_of(packet.payload())).ok();
            self.client.send_packet(&packet);
            let response: Packet<WriteFileResponse> = self.client.read_packet()?;
            if !response.payload().is_ok() {
                return None;
            }
        }

        Some(())
    }

    pub fn handle(&self) -> &FileHandle {
        &self.handle
    }

    pub fn open_mode(&self) -> &FileFlags {
        &self.open_mode
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::bytes_of;

    use crate::{
        client::{cfg::set_packet_mode, serial::tests::TestSerialWrapper},
        common::{
            commands::{
                FileFlags, FileHandle, FileResponse, OpenFile, WriteFile, WriteFileResponse,
            },
            packet::Packet,
            PacketContents,
        },
    };

    use super::File;

    #[test]
    fn test_create_file() {
        set_packet_mode(true);
        let serial = TestSerialWrapper::new();
        let file_resp = FileResponse::new(0x1234).into_packet();
        serial.get_adapter().set_input(bytes_of(&file_resp));
        let file = File::create_file_with(&serial, "test.txt");
        assert!(file.is_some());
        let file = file.unwrap();
        assert_eq!(file.handle(), &FileHandle::new(0x1234));
        assert_eq!(file.open_mode(), &FileFlags::CREATE_OVERWRITE);
        serial.get_adapter().assert_send(OpenFile::PACKET_SIZE);
        serial.get_adapter().assert_read(FileResponse::PACKET_SIZE);
    }

    #[test]
    fn test_write_file() {
        set_packet_mode(true);
        let serial = TestSerialWrapper::new();
        let file_resp = FileResponse::new(0x1234).into_packet();
        let write_resp = WriteFileResponse::ok().into_packet();
        let mut data = Vec::new();
        data.extend_from_slice(bytes_of(&file_resp));
        data.extend_from_slice(bytes_of(&write_resp));

        serial.get_adapter().set_input(&data);

        let file = File::create_file_with(&serial, "test.txt").expect("Create file failed");
        assert_eq!(file.handle(), &FileHandle::new(0x1234));
        assert_eq!(file.open_mode(), &FileFlags::CREATE_OVERWRITE);
        assert!(file.write(b"Hello, world!").is_some());
        serial
            .get_adapter()
            .assert_send(OpenFile::PACKET_SIZE + WriteFile::PACKET_SIZE);
        serial
            .get_adapter()
            .assert_read(FileResponse::PACKET_SIZE + WriteFileResponse::PACKET_SIZE);
    }
}
