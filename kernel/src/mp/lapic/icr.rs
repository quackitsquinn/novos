use bitfield::bitfield;
use core::fmt::Debug;

use crate::mp::id;

bitfield! {
    /// Interrupt Command Register (ICR) register.
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct InterruptCommandRegister(u64);
    impl Debug;
    /// The interrupt vector to send.
    pub u8, vector, set_vector: 7, 0;
    /// The delivery mode of the interrupt.
    pub u8, from DeliverMode, delivery_mode, set_delivery_mode: 10, 8;
    /// The destination mode of the interrupt (0 = physical, 1 = logical).
    pub bool, destination_mode, set_destination_mode: 11;
    /// The delivery status of the interrupt (0 = idle, 1 = send pending).
    pub bool, delivery_status, _: 12;
    /// The level of the interrupt (0 = deassert, 1 = assert).
    /// Because limine does the initialization for each AP, we will always be asserting.
    pub bool, level, set_level: 14;
    /// The trigger mode of the interrupt (0 = edge, 1 = level).
    pub bool, trigger_mode, set_trigger_mode: 15;
    /// An optional shorthand notation for the destination field.
    pub u8, from DestinationShorthand, destination_shorthand, set_destination_shorthand: 19, 18;
    /// The destination field specifying the target processor(s).
    pub u8, destination, set_destination: 63, 56;
}

id!(InterruptCommandRegister, REGISTER, 0x300);

/// Delivery mode of the interrupt.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl From<DeliverMode> for u8 {
    fn from(deliver: DeliverMode) -> u8 {
        deliver as u8
    }
}

/// An optional shorthand notation for the destination field.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl From<DestinationShorthand> for u8 {
    fn from(shorthand: DestinationShorthand) -> u8 {
        shorthand as u8
    }
}
