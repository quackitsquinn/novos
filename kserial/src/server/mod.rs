use std::io::{self};

use serial_stream::SerialStream;

use crate::common::{packet::Packet, PacketContents};

mod copy_rw;
mod handlers;
pub mod serial_handler;
pub(crate) mod serial_stream;

pub use serial_handler::SerialHandler;

pub(crate) fn read_packet<C: PacketContents>(
    cmd_id: u8,
    stream: &mut SerialStream,
) -> Result<Packet<C>, io::Error> {
    let checksum = stream.read_ty::<u8>()?;
    let packet = stream.read_ty::<C>()?;
    let full = Packet::from_raw_parts(cmd_id, checksum, packet).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Checksum mismatch, expected 0 but got {}",
            packet.checksum()
        ),
    ))?;
    Ok(full)
}
