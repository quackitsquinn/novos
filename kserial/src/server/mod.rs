//! Server module for kserial. Handles incoming commands over serial.
//! Not compatible with `no_std` environments.
mod copy_rw;
mod handlers;
pub(crate) mod packet_error;
pub mod serial_handler;
pub(crate) mod serial_stream;

use packet_error::PacketError;
pub use serial_handler::SerialHandler;

/// If set, the server will panic on checksum mismatches instead of returning an error.
pub const PANIC_ON_CHECKSUM_MISMATCH: bool = option_env!("PANIC_ON_CHECKSUM_MISMATCH").is_some();

fn handle_invalid_checksum(_: u8) -> PacketError {
    if PANIC_ON_CHECKSUM_MISMATCH {
        panic!("Checksum mismatch!");
    } else {
        PacketError::InvalidChecksum
    }
}
