pub mod serial_adapter;

pub use serial_adapter::{SerialAdapter, WouldBlockError};
use spin::Once;

static SERIAL_ADAPTER: Once<&'static dyn SerialAdapter> = Once::new();

pub fn init(adapter: &'static dyn SerialAdapter) {
    SERIAL_ADAPTER.call_once(|| adapter);
}
