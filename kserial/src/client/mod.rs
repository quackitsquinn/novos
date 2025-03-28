pub mod cfg;
pub mod serial_adapter;

pub use serial_adapter::SerialAdapter;
use spin::Once;


use crate::common::{commands::StringPacket, PacketContents};

static SERIAL_ADAPTER: Once<&'static dyn SerialAdapter> = Once::new();

pub fn init(adapter: &'static dyn SerialAdapter) {
    SERIAL_ADAPTER.call_once(|| adapter);
}
/// Sends a POD type over serial.
///
/// # Safety
///
/// The caller must ensure that the receiver is expecting the same type.
pub(crate) unsafe fn send_pod<T>(data: &T)
where
    T: bytemuck::Pod,
{
    if !cfg::should_output_serial() {
        return;
    }
    // We allow sending POD types over serial, even if we are not in packet mode.
    let adapter = SERIAL_ADAPTER
        .get()
        .expect("Serial adapter not initialized");
    let bytes = bytemuck::bytes_of(data);
    adapter.send_slice(bytes);
}

pub fn send_string(string: &str) {
    if !cfg::is_packet_mode() {
        let adapter = SERIAL_ADAPTER
            .get()
            .expect("Serial adapter not initialized");
        for byte in string.bytes() {
            adapter.send(byte);
        }
        return;
    }

    for chunk in string.as_bytes().chunks(StringPacket::CAPACITY) {
        let pk = unsafe { StringPacket::from_bytes_unchecked(chunk) };
        unsafe {
            send_pod(&pk.into_packet());
        }
    }
}
