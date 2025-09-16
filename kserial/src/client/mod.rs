pub mod cfg;
pub mod fs;
pub mod serial;
pub mod serial_adapter;

use core::fmt;

use serial::SerialClient;
pub use serial_adapter::SerialAdapter;

use crate::common::{commands::StringPacket, packet::Packet, PacketContents};

// TODO: This crate assumes there is a serial connection that works 2 way. This is not always true. We should add a way to test this at some point.

static SERIAL_ADAPTER: SerialClient = SerialClient::new();

pub fn init(adapter: &'static mut dyn SerialAdapter) {
    SERIAL_ADAPTER.init(adapter);
}

pub fn get_serial_client() -> &'static SerialClient<'static> {
    // This is used to get the serial client for other modules.
    &SERIAL_ADAPTER
}

pub fn send_string(string: &str) {
    send_string_with(&SERIAL_ADAPTER, string);
}

pub fn send_string_with<'a>(serial: &'a SerialClient<'a>, string: &str) {
    let mut lock = serial
        .lock()
        .expect("Serial client not initialized, cannot send string.");
    let serial_adapter = lock
        .get_adapter()
        .as_mut()
        .expect("Serial adapter not initialized, cannot send string.");

    if !cfg::is_packet_mode() {
        serial_adapter.send_slice(string.as_bytes());
        return;
    }

    for chunk in string.as_bytes().chunks(StringPacket::CAPACITY) {
        let pk = unsafe { StringPacket::from_bytes_unchecked(chunk) };

        let packet = pk.into_packet();
        lock.send_packet(&packet);
    }
}

pub fn test_two_way_serial() {
    let serial = &SERIAL_ADAPTER;
    let packet = StringPacket::new("Hello, world!").unwrap();
    let echo_packet = unsafe { Packet::new(0xFE, packet) };
    let mut session = serial.lock().expect("Serial not initialized");
    session.send_packet(&echo_packet);
    let echoed_packet: Packet<StringPacket> = session.read_packet().expect("Failed to read packet");
    assert_eq!(echoed_packet.command(), 0xFE);
    assert_eq!(echoed_packet.checksum(), 0);
    assert_eq!(echoed_packet.payload().as_str(), "Hello, world!");
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
    use std::io::{self, Cursor};

    use crate::{
        common::{commands::StringPacket, packet::Packet, PacketContents},
        server::serial_stream::SerialStream,
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

        let mut ser = SerialStream::new(cur, io::stdout());

        let packet: Packet<StringPacket> = ser.read_packet(StringPacket::ID).unwrap();
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
