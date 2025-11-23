//! Interrupt Command Register (ICR) interface for LAPIC.
use bitfield::bitfield;

use core::fmt::Debug;

use crate::{interrupts::KernelInterrupt, mp::id};

bitfield! {
    /// Interrupt Command Register (ICR) register.
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct InterruptCommandRegisterValue(u64);
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

id!(InterruptCommandRegisterValue, REGISTER, 0x300);

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

pub struct InterruptCommandRegister<'a> {
    lapic: &'a super::Lapic,
}

impl<'a> InterruptCommandRegister<'a> {
    /// Creates a new ICR interface for the given LAPIC.
    pub(super) const fn new(lapic: &'a super::Lapic) -> Self {
        Self { lapic }
    }

    /// Writes to the Interrupt Command Register (ICR).
    ///
    /// # Safety
    /// The caller must ensure that the given ICR value is valid.
    /// The caller must also ensure that the deliver status is not modified.
    #[must_use = "The returned PossiblyPending should be used to check/wait for IPI delivery"]
    pub unsafe fn write(&self, icr: InterruptCommandRegisterValue) -> PossiblyPending<'_> {
        let icr: u64 = icr.0;
        let low: u32 = (icr & 0xFFFF_FFFF) as u32;
        let high: u32 = (icr >> 32) as u32;
        unsafe {
            self.lapic
                .write_offset::<u32>(InterruptCommandRegisterValue::REGISTER + 0x10, high);
            self.lapic
                .write_offset::<u32>(InterruptCommandRegisterValue::REGISTER, low);
        }

        PossiblyPending::new(self)
    }

    // Split read functions for low, high, and full ICR, mainly to reduce unneeded MMIO reads.
    // Notably, we don't do this in writes because the ICR should always be written in full.

    /// Reads the low 32 bits of the Interrupt Command Register (ICR).
    ///
    /// This will zero-extend the value to 64 bits.
    pub fn read_low(&self) -> InterruptCommandRegisterValue {
        let raw_value = unsafe {
            self.lapic
                .read_offset::<u32>(InterruptCommandRegisterValue::REGISTER)
        };
        InterruptCommandRegisterValue(raw_value as u64)
    }

    /// Reads the high 32 bits of the Interrupt Command Register (ICR).
    ///
    /// This will shift the value to the high 32 bits of a 64-bit value.
    pub fn read_high(&self) -> InterruptCommandRegisterValue {
        let raw_value = unsafe {
            self.lapic
                .read_offset::<u32>(InterruptCommandRegisterValue::REGISTER + 0x10)
        };
        InterruptCommandRegisterValue((raw_value as u64) << 32)
    }

    /// Reads the full 64 bits of the Interrupt Command Register (ICR).
    ///
    /// This combines the low and high reads.
    pub fn read_all(&self) -> InterruptCommandRegisterValue {
        let low = self.read_low().0;
        let high = self.read_high().0;
        InterruptCommandRegisterValue(low | high)
    }

    /// Sends an IPI to the specified destination with the given interrupt vector.
    #[must_use = "The returned PossiblyPending should be used to check/wait for IPI delivery"]
    pub fn send(&self, dest: IPIDestination, vector: KernelInterrupt) -> PossiblyPending<'_> {
        let mut icr = InterruptCommandRegisterValue(0);

        icr.set_vector(vector as u8);
        icr.set_delivery_mode(DeliverMode::Fixed);
        icr.set_destination_mode(false); // Physical mode
        icr.set_level(true); // Assert
        icr.set_trigger_mode(false); // Edge triggered

        // Set the destination based on the specified type.
        match dest {
            IPIDestination::AllCores => {
                icr.set_destination_shorthand(DestinationShorthand::AllIncludingSelf);
            }
            IPIDestination::AllExceptSelf => {
                icr.set_destination_shorthand(DestinationShorthand::AllExcludingSelf);
            }
            IPIDestination::SelfOnly => {
                icr.set_destination_shorthand(DestinationShorthand::SelfOnly);
            }
            IPIDestination::Physical(apic_id) => {
                icr.set_destination(apic_id);
            }
            IPIDestination::Logical(logical_id) => {
                icr.set_destination_mode(true); // Logical mode
                icr.set_destination(logical_id);
            }
        }

        // Write to the ICR registers to send the IPI.
        unsafe { self.write(icr) }
    }
}

pub struct PossiblyPending<'a> {
    icr: &'a InterruptCommandRegister<'a>,
}
impl<'a> PossiblyPending<'a> {
    /// Creates a new interface for checking if an IPI is possibly pending.
    pub const fn new(icr: &'a InterruptCommandRegister<'a>) -> Self {
        Self { icr }
    }

    /// Checks if an IPI is pending.
    pub fn is_pending(&self) -> bool {
        let icr_value = self.icr.read_low();
        let pending = icr_value.delivery_status();
        pending
    }

    /// Waits until the IPI is no longer pending.
    pub fn wait(&self) {
        while self.is_pending() {
            core::hint::spin_loop();
        }
    }

    /// Ignores the pending status of the IPI.
    pub fn ignore(self) {}
}

/// The destination for an IPI.
#[derive(Debug, Clone, Copy)]
pub enum IPIDestination {
    /// Send to all cores including self.
    AllCores,
    /// Send to all cores except self.
    AllExceptSelf,
    /// Send to only self.
    SelfOnly,
    /// Send to a specific core by its APIC ID.
    Physical(u8),
    /// Send to a specific logical core ID.
    Logical(u8),
}
