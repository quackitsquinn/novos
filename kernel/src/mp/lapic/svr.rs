use core::fmt::Debug;
use modular_bitfield::prelude::*;

#[derive(Clone, Copy)]
#[bitfield(bytes = 4)]
pub struct SpuriousInterruptVector {
    pub vector: B8,
    pub apic_enable: bool,
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
