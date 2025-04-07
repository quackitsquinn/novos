pub mod cfg;
pub mod serial;
pub mod serial_adapter;

use core::fmt;

use serial::SerialClient;
pub use serial_adapter::SerialAdapter;
use spin::Once;

use crate::common::{commands::StringPacket, test_log::info, PacketContents};

static SERIAL_ADAPTER: SerialClient = SerialClient::new();

pub fn init(adapter: &'static dyn SerialAdapter) {
    SERIAL_ADAPTER.init(adapter);
}

pub fn get_serial_client() -> &'static SerialClient {
    // This is used to get the serial client for other modules.
    &SERIAL_ADAPTER
}

pub fn send_string(string: &str) {
    send_string_with(&SERIAL_ADAPTER, string);
}

pub fn send_string_with(serial: &SerialClient, string: &str) {
    let serial_adapter = serial.get().expect("Serial adapter not initialized");
    if !cfg::is_packet_mode() {
        serial_adapter.send_slice(string.as_bytes());
        return;
    }

    for chunk in string.as_bytes().chunks(StringPacket::CAPACITY) {
        let pk = unsafe { StringPacket::from_bytes_unchecked(chunk) };
        unsafe {
            let packet = pk.into_packet();
            info!("Sending packet: {:?}", packet);
            serial.send_pod(&packet);
        }
    }
}

pub struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        send_string(s);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        common::{commands::StringPacket, packet::Packet, PacketContents},
        server::{read_packet, serial_stream::SerialStream},
    };

    use super::{cfg, serial::tests::TestSerialWrapper};

    #[test]
    fn test_send_string() {
        const TEST_STRING: &str = "Hello, world! hEllo, world! heLlo, world!";
        let tester = TestSerialWrapper::new();
        let serial = &tester.serial;
        let adapter = &tester.get_adapter();

        cfg::set_packet_mode(true);
        super::send_string_with(serial, TEST_STRING);
        let output = adapter.get_output();
        let cur = Cursor::new(output[1..].to_vec());
        assert!(
            output[0] == StringPacket::ID as u8,
            "Invalid packet ID, Expected: {}, Got: {}: {:?}",
            StringPacket::ID,
            output[0],
            output
        );

        let mut ser = SerialStream::new(cur);

        let packet: Packet<StringPacket> = read_packet(StringPacket::ID, &mut ser).unwrap();
        assert_eq!(packet.command(), StringPacket::ID);
        assert_eq!(packet.checksum(), 0);
        let contents = packet.payload();
        assert_eq!(
            contents.as_str(),
            &TEST_STRING[..StringPacket::CAPACITY],
            "Failed to read the correct string from the packet."
        );

        //let remaining_output = read_packet(Str, stream)
    }
}
