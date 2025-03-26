pub mod serial_adapter;

pub use serial_adapter::SerialAdapter;
use spin::Once;

use core::fmt::Write;

static SERIAL_ADAPTER: Once<&'static dyn SerialAdapter> = Once::new();

pub fn init(adapter: &'static dyn SerialAdapter) {
    SERIAL_ADAPTER.call_once(|| adapter);
}

pub(crate) unsafe fn send_pod<T>(data: &T)
where
    T: bytemuck::Pod,
{
    let adapter = SERIAL_ADAPTER
        .get()
        .expect("Serial adapter not initialized");
    let bytes = bytemuck::bytes_of(data);
    adapter.send_slice(bytes);
}
