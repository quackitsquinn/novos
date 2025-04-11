use bytemuck::{Pod, Zeroable};

use super::PacketContents;

mod file;
mod incremental;
mod string_packet;

pub use file::{FileFlags, FileHandle, FileResponse, OpenFile};
pub use incremental::{CloseIncrementalFileChannel, CreateIncrementalFileChannel, IncrementalFile};
pub use string_packet::StringPacket;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Shutdown {
    pub code: i32,
}

impl PacketContents for Shutdown {
    const ID: u8 = 0x09;
}

impl Shutdown {
    pub fn new(code: i32) -> Self {
        Self { code }
    }
}
