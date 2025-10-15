use core::fmt::Debug;
use modular_bitfield::prelude::*;

use crate::mp::id;

/// Represents the LAPIC version register.
#[derive(Clone, Copy)]
#[bitfield(bytes = 4)]
pub struct LapicVersion {
    pub version: u8,
    #[skip]
    __: B8,
    pub max_lvt_entry: B7,
    pub supports_eoi_broadcast_suppression: bool,
    #[skip]
    __: B8,
}

id!(LapicVersion, REGISTER, 0x30);

impl Debug for LapicVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LapicVersion")
            .field("version", &self.version())
            .field("max_lvt_entry", &self.max_lvt_entry())
            .field(
                "supports_eoi_broadcast_suppression",
                &self.supports_eoi_broadcast_suppression(),
            )
            .finish()
    }
}
