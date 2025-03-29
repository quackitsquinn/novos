pub mod cfg;
pub mod serial;
pub mod serial_adapter;

use serial::Serial;
pub use serial_adapter::SerialAdapter;
use spin::Once;

use crate::common::{commands::StringPacket, test_log::info, PacketContents};

static SERIAL_ADAPTER: Serial = Serial::new();

pub fn init(adapter: &'static dyn SerialAdapter) {
    SERIAL_ADAPTER.init(adapter);
}

pub fn send_string(string: &str) {
    send_string_with(&SERIAL_ADAPTER, string);
}

pub fn send_string_with(serial: &Serial, string: &str) {
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        common::{commands::StringPacket, packet::Packet, PacketContents},
        server::read_packet,
    };

    use super::{cfg, serial::tests::TestSerialWrapper};

    #[test]
    fn test_send_string() {
        let tester = TestSerialWrapper::new();
        let serial = &tester.serial;
        let adapter = &tester.get_adapter();

        cfg::set_packet_mode(true);
        super::send_string_with(serial, "Hello, world!");
        let output = adapter.get_output();
        let mut cur = Cursor::new(output[1..].to_vec());
        assert!(
            output[0] == StringPacket::ID as u8,
            "Invalid packet ID, Expected: {}, Got: {}: {:?}",
            StringPacket::ID,
            output[0],
            output
        );

        let packet: Packet<StringPacket> = read_packet(StringPacket::ID, &mut cur).unwrap();
        assert_eq!(packet.command(), StringPacket::ID);
    }
}
