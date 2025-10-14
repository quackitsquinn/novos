use core::fmt::Debug;
use modular_bitfield::prelude::*;

/// Spurious Interrupt Vector Register (SVR).
#[derive(Clone, Copy)]
#[bitfield(bytes = 4)]
pub struct SpuriousInterruptVector {
    /// Determines the vector number to be delivered to the processor when the local APIC generates a spurious vector.
    pub vector: B8,
    /// Allows software to temporarily enable (1) or disable (0) the local APIC
    pub apic_enable: bool,
    /// Determines if focus processor checking is enabled (0) or disabled (1) when using the lowest-priority delivery mode.
    pub focus_processor_checking: bool,
    #[skip]
    __: B6,
    pub eoi_broadcast_suppression: bool,
    #[skip]
    __: B15,
}

impl Debug for SpuriousInterruptVector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SpuriousInterruptVector")
            .field("vector", &self.vector())
            .field("apic_enable", &self.apic_enable())
            .field("focus_processor_checking", &self.focus_processor_checking())
            .field(
                "eoi_broadcast_suppression",
                &self.eoi_broadcast_suppression(),
            )
            .finish()
    }
}
