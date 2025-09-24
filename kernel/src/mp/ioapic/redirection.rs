use core::fmt::Debug;
use modular_bitfield::prelude::*;

#[bitfield(bytes = 8)]
pub struct RedirectionEntry {
    interrupt_vector: B8,
    #[bits = 3]
    delivery_mode: DeliveryMode,
    destination_mode: bool,
    delivery_status: bool,
    polarity: bool,
    remote_irr: bool,
    trigger_mode: bool,
    mask: bool,
    #[skip]
    __: B39,
    destination: B8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Specifier)]
pub enum DeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    ExtInt = 0b111,
    Invalid1 = 0b011,
    Invalid2 = 0b110,
}

impl Debug for RedirectionEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RedirectionEntry")
            .field("interrupt_vector", &self.interrupt_vector())
            .field("delivery_mode", &self.delivery_mode())
            .field("destination_mode", &self.destination_mode())
            .field("delivery_status", &self.delivery_status())
            .field("polarity", &self.polarity())
            .field("remote_irr", &self.remote_irr())
            .field("trigger_mode", &self.trigger_mode())
            .field("mask", &self.mask())
            .field("destination", &self.destination())
            .finish()
    }
}
