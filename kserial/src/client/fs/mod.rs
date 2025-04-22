use err::FileError;


use crate::common::{
    commands::{FileFlags, FileHandle, FileResponse, OpenFile, WriteFile, WriteFileResponse},
    packet::Packet,
    PacketContents,
};

use super::{cfg::is_packet_mode, send_string, serial::SerialClient};

pub mod err;

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

    pub fn create_file_with(serial: &'a SerialClient, name: &str) -> Result<Self, FileError> {
        let open_file = OpenFile::create(name).ok_or(FileError::FilenameTooLong)?;
        // Copy the flags so that if the flags for `create` are changed, we don't have to change this function.
        let flags = open_file.flags;

        // TODO: Allow these values to be overridden by the inner SerialAdapter type
        if !is_packet_mode() {
            // TODO: User configurable error handling / Error type
            //send_string("Packet mode is not enabled, cannot create file.");
            return Err(FileError::NotInPacketMode);
        }

        serial.send_packet(&open_file.into_packet());
        let response: Packet<FileResponse> = serial.read_packet().ok_or(FileError::ReadError)?;
        let response = response.payload();

        if !response.err.is_ok() {
            return Err(FileError::IoError(response.err));
        }

        let handle = response.handle;

        let file = unsafe { File::new(handle, flags, serial) };
        Ok(file)
    }

    pub fn create_file(name: &str) -> Result<Self, FileError> {
        Self::create_file_with(&super::SERIAL_ADAPTER, name)
    }

    pub fn write(&self, data: &[u8]) -> Result<(), FileError> {
        if !is_packet_mode() {
            send_string("Packet mode is not enabled, cannot write to file.");
            return Err(FileError::NotInPacketMode);
        }

        for chunk in data.chunks(WriteFile::CAPACITY) {
            let write_file = WriteFile::new(self.handle, chunk).expect("infallible");
            let packet = write_file.into_packet();
            // writeln!(SerialWriter, "{:?}", packet).ok();
            // writeln!(SerialWriter, "{:?}", bytemuck::bytes_of(packet.payload())).ok();
            self.client.send_packet(&packet);
            let response: Packet<WriteFileResponse> =
                self.client.read_packet().ok_or(FileError::ReadError)?;
            if !response.payload().is_ok() {
                return Err(FileError::IoError(response.payload().err));
            }
        }

        Ok(())
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
    

    use crate::{
        client::{cfg::set_packet_mode, serial::tests::TestSerialWrapper},
        common::{
            commands::{
                FileFlags, FileHandle, FileResponse, OpenFile, WriteFile, WriteFileResponse,
            },
            PacketContents,
        },
    };

    use super::File;

    #[test]
    fn test_create_file() {
        set_packet_mode(true);
        let serial = TestSerialWrapper::new();
        let file_resp = FileResponse::new(0x1234).into_packet();
        serial.get_adapter().set_input(&file_resp.as_bytes());
        let file = File::create_file_with(&serial, "test.txt");
        assert!(file.is_ok());
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
        data.extend_from_slice(&file_resp.as_bytes());
        data.extend_from_slice(&write_resp.as_bytes());

        serial.get_adapter().set_input(&data);

        let file = File::create_file_with(&serial, "test.txt").expect("Create file failed");
        assert_eq!(file.handle(), &FileHandle::new(0x1234));
        assert_eq!(file.open_mode(), &FileFlags::CREATE_OVERWRITE);
        assert!(file.write(b"Hello, world!").is_ok());
        serial
            .get_adapter()
            .assert_send(OpenFile::PACKET_SIZE + WriteFile::PACKET_SIZE);
        serial
            .get_adapter()
            .assert_read(FileResponse::PACKET_SIZE + WriteFileResponse::PACKET_SIZE);
    }
}
