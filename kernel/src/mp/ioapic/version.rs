use core::fmt::Debug;

use modular_bitfield::prelude::*;

/// Represents the version register of an IOAPIC.
#[bitfield(bytes = 4)]
pub struct IoApicVersion {
    /// The version of the IOAPIC.
    pub version: B8,
    #[skip]
    __: B8,
    /// The maximum number of redirection entries supported by the IOAPIC.
    pub max_redirection_entries: B8,
    #[skip]
    __: B8,
}

impl IoApicVersion {
    /// The IOAPIC version register address.
    pub const REGISTER: u8 = 0x01;
}

impl Debug for IoApicVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IoApicVersion")
            .field("version", &self.version())
            .field("max_redirection_entries", &self.max_redirection_entries())
            .finish()
    }
}
