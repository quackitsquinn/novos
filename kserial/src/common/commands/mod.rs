//! Commands that can be sent over the serial port.
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

/// A command to shut down the system.
#[derive(Debug, Clone, Copy, Pod, Zeroable, Validate)]
#[repr(C)]
pub struct Shutdown {
    /// The shutdown code.
    pub code: i32,
}

impl PacketContents for Shutdown {
    const ID: u8 = SHUTDOWN_ID;
}

impl Shutdown {
    /// Create a new `Shutdown` command.
    pub fn new(code: i32) -> Self {
        Self { code }
    }
}
