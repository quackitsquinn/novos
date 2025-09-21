use core::fmt::Debug;
use modular_bitfield::prelude::*;

#[derive(Clone, Copy)]
#[bitfield(bytes = 8)]
pub struct InterruptCommandRegister {
    /// The interrupt vector to send.
    pub vector: B8,
    /// The delivery mode of the interrupt.
    #[bits = 3]
    pub delivery_mode: DeliverMode,
    /// The destination mode of the interrupt (0 = physical, 1 = logical).
    pub destination_mode: bool,
    /// The delivery status of the interrupt (0 = idle, 1 = send pending).
    pub delivery_status: bool,
    /// The level of the interrupt (0 = deassert, 1 = assert).
    pub level: bool,
    /// The trigger mode of the interrupt (0 = edge, 1 = level).
    pub trigger_mode: bool,
    #[skip]
    __: B2,
    /// The destination shorthand of the interrupt.
    pub destination_shorthand: DestinationShorthand,
    #[skip]
    __: B37,
    /// The destination field of the interrupt (only used if destination shorthand is 0).
    pub destination: B8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Specifier)]
pub enum DeliverMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    Startup = 0b110,
    Invalid1 = 0b011,
    Invalid2 = 0b111,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Specifier)]
pub enum DestinationShorthand {
    NoShorthand = 0b00,
    SelfOnly = 0b01,
    AllIncludingSelf = 0b10,
    AllExcludingSelf = 0b11,
}

impl Debug for InterruptCommandRegister {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InterruptCommandRegister")
            .field("vector", &self.vector())
            .field("delivery_mode", &self.delivery_mode())
            .field("destination_mode", &self.destination_mode())
            .field("delivery_status", &self.delivery_status())
            .field("level", &self.level())
            .field("trigger_mode", &self.trigger_mode())
            .field("destination_shorthand", &self.destination_shorthand())
            .field("destination", &self.destination())
            .finish()
    }
}
