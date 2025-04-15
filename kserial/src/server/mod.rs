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

pub(crate) fn read_packet<C: PacketContents + Debug>(
    cmd_id: u8,
    stream: &mut SerialStream,
) -> Result<Packet<C>, io::Error> {
    let checksum = stream.read_ty::<u8>()?;
    let packet = stream.read_ty::<C>()?;
    if TypeId::of::<C>() == TypeId::of::<WriteFile>() {
        println!("Received packet: {packet:?}");
        println!("As bytes: {:?}", bytemuck::bytes_of(&packet));
    }
    let full = Packet::from_raw_parts(cmd_id, checksum, packet)
        .ok_or_else(|| handle_invalid_checksum(packet.checksum()))?;
    Ok(full)
}

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
