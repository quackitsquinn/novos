use core::{any::TypeId, fmt::Debug};
use std::io::{self};

use serial_stream::SerialStream;

use crate::common::{commands::WriteFile, packet::Packet, PacketContents};

mod copy_rw;
mod handlers;
pub mod serial_handler;
pub(crate) mod serial_stream;

pub use serial_handler::SerialHandler;

pub const PANIC_ON_CHECKSUM_MISMATCH: bool = option_env!("PANIC_ON_CHECKSUM_MISMATCH").is_some();

fn handle_invalid_checksum(checksum: u8) -> io::Error {
    if PANIC_ON_CHECKSUM_MISMATCH {
        panic!("Checksum mismatch!");
    } else {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Checksum mismatch, expected 0 but got {}", checksum),
        )
    }
}
