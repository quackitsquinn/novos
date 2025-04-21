use bytemuck::{Pod, Zeroable};
use ids::SHUTDOWN_ID;
use kserial_derive::Validate;

use super::PacketContents;

mod file;
mod incremental;
mod string_packet;

pub use file::*;
pub use incremental::{CloseIncrementalFileChannel, CreateIncrementalFileChannel, IncrementalFile};
pub use string_packet::StringPacket;
pub mod ids;

#[derive(Debug, Clone, Copy, Pod, Zeroable, Validate)]
#[repr(C)]
pub struct Shutdown {
    pub code: i32,
}

impl PacketContents for Shutdown {
    const ID: u8 = SHUTDOWN_ID;
}

impl Shutdown {
    pub fn new(code: i32) -> Self {
        Self { code }
    }
}
