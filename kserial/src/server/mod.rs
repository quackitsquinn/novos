
mod copy_rw;
mod handlers;
pub(crate) mod packet_error;
pub mod serial_handler;
pub(crate) mod serial_stream;

use packet_error::PacketError;
pub use serial_handler::SerialHandler;

pub const PANIC_ON_CHECKSUM_MISMATCH: bool = option_env!("PANIC_ON_CHECKSUM_MISMATCH").is_some();

fn handle_invalid_checksum(checksum: u8) -> PacketError {
    if PANIC_ON_CHECKSUM_MISMATCH {
        panic!("Checksum mismatch!");
    } else {
        PacketError::InvalidChecksum
    }
}
