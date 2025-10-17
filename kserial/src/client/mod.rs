//! Client module for kserial. Compatible with `no_std` environments.
pub mod cfg;
pub mod fs;
pub mod serial;
pub mod serial_adapter;

use core::fmt::{self, Debug};

use serial::SerialClient;
pub use serial_adapter::SerialAdapter;

use crate::{
    client::serial::AdapterContainer,
    common::{commands::StringPacket, PacketContents},
};

// TODO: This crate assumes there is a serial connection that works 2 way. This is not always true. We should add a way to test this at some point.

static SERIAL_ADAPTER: SerialClient = SerialClient::new();

/// Initialize the global serial adapter.
pub fn init(adapter: &'static mut dyn SerialAdapter) {
    SERIAL_ADAPTER.init(adapter);
}

/// Get the global serial client.
pub fn get_serial_client() -> &'static SerialClient<'static> {
    // This is used to get the serial client for other modules.
    &SERIAL_ADAPTER
}

/// Send a string over the serial connection.
pub fn send_string(string: &str) {
    send_string_with(&mut SERIAL_ADAPTER.lock().expect("uninit"), string);
}

/// Send a string over the serial connection using the given adapter lock.
pub fn send_string_with<'a>(lock: &mut AdapterContainer<'a>, string: &str) {
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

/// A writer that writes to the serial connection.
pub struct SerialWriter<'a>(AdapterContainer<'a>);

impl<'a> fmt::Write for SerialWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        send_string_with(&mut self.0, s);
        Ok(())
    }
}

impl Debug for SerialWriter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SerialWriter").finish()
    }
}

/// Get a SerialWriter that writes to the global serial adapter.
pub fn writer() -> SerialWriter<'static> {
    SerialWriter(SERIAL_ADAPTER.lock().expect("Serial not initialized"))
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
        super::send_string_with(&mut serial.lock().expect("uninit"), TEST_STRING);
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
