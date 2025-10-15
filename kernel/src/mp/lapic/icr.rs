use core::fmt::Debug;
use modular_bitfield::prelude::*;

use crate::mp::id;

/// Interrupt Command Register (ICR) register.
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

id!(InterruptCommandRegister, REGISTER, 0x300);

/// Delivery mode of the interrupt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Specifier)]
pub enum DeliverMode {
    /// Delivers the interrupt specified in the vector field to the target processor
    /// or processors.
    Fixed = 0b000,
    /// Same as fixed mode, except that the interrupt is delivered to the proces-
    /// sor executing at the lowest priority among the set of processors specified
    /// in the destination field. The ability for a processor to send a lowest priority
    /// IPI is model specific and should be avoided by BIOS and operating system
    /// software
    LowestPriority = 0b001,
    /// Delivers an SMI interrupt to the target processor or processors. The vector
    /// field must be programmed to 00H for future compatibility.
    Smi = 0b010,
    /// Delivers an NMI interrupt to the target processor or processors. The vector
    /// information is ignored.
    Nmi = 0b100,
    /// Delivers an INIT request to the target processor or processors, which
    /// causes them to perform an INIT. As a result of this IPI message, all the
    /// target processors perform an INIT. The vector field must be programmed
    /// to 00H for future compatibility.
    Init = 0b101,
    /// Sends a special “start-up” IPI (called a SIPI) to the target processor or
    /// processors. The vector typically points to a start-up routine that is part of
    /// the BIOS boot-strap code (see Section 8.4, “Multiple-Processor (MP) Ini-
    /// tialization”). IPIs sent with this delivery mode are not automatically retried
    /// if the source APIC is unable to deliver it. It is up to the software to deter-
    /// mine if the SIPI was not successfully delivered and to reissue the SIPI if
    /// necessary.
    Startup = 0b110,
    #[doc(hidden)]
    _Invalid1 = 0b011,
    #[doc(hidden)]
    _Invalid2 = 0b111,
}

/// An optional shorthand notation for the destination field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Specifier)]
pub enum DestinationShorthand {
    /// No shorthand notation. The destination field contains the destination
    NoShorthand = 0b00,
    /// The issuer is the sole recipient of the interrupt.
    SelfOnly = 0b01,
    /// All processors including the issuer are the recipients of the interrupt.
    AllIncludingSelf = 0b10,
    /// All processors excluding the issuer are the recipients of the interrupt.
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
