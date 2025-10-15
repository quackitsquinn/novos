//! Redirection entry.
use core::fmt::Debug;
use modular_bitfield::prelude::*;

/// Represents a redirection entry in the IOAPIC.
#[bitfield(bytes = 8)]
pub struct RedirectionEntry {
    /// The vector number of the interrupt being sent
    interrupt_vector: B8,
    /// The delivery mode of the interrupt
    #[bits = 3]
    delivery_mode: DeliveryMode,
    /// This field determines the interpretation of the Destination field.
    /// When DESTMOD=0 (physical mode), a destination APIC is identified by its ID.
    /// Bits 56 through 59 of the Destination field specify the 4 bit APIC ID. When DESTMOD=1 (logical
    /// mode), destinations are identified by matching on the logical destination under the control of the
    /// Destination Format Register and Logical Destination Register in each Local APIC.
    destination_mode: bool,
    /// The Delivery Status bit contains the current status of the
    /// delivery of this interrupt. Delivery Status is read-only and writes to this bit (as part of a 32 bit
    /// word) do not effect this bit.
    delivery_status: bool,
    /// This bit specifies the polarity of the interrupt signal. 0=High active, 1=Low active.
    polarity: bool,
    /// This bit is used for level triggered interrupts. Its meaning is undefined for
    /// edge triggered interrupts. For level triggered interrupts, this bit is set to 1 when local APIC(s)
    /// accept the level interrupt sent by the IOAPIC. The Remote IRR bit is set to 0 when an EOI
    /// message with a matching interrupt vector is received from a local APIC.
    remote_irr: bool,
    /// The trigger mode field indicates the type of signal on the interrupt pin that
    ///triggers an interrupt. 1=Level sensitive, 0=Edge sensitive.
    trigger_mode: bool,
    /// When this bit is 1, the interrupt signal is masked. Edge-sensitive
    /// interrupts signaled on a masked interrupt pin are ignored (i.e., not delivered or held pending).
    /// Level-asserts or negates occurring on a masked level-sensitive pin are also ignored and have no
    /// side effects.
    mask: bool,
    #[skip]
    __: B39,
    /// The Destination Mode of this entry is Physical Mode (bit 11=0), bits
    /// [59:56] contain an APIC ID. If Logical Mode is selected (bit 11=1), the Destination Field
    /// potentially defines a set of processors. Bits [63:56] of the Destination Field specify the logical
    /// destination address.
    destination: B8,
}

/// Delivery types of IPIs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Specifier)]
pub enum DeliveryMode {
    /// Deliver the signal on the INTR signal of all processor cores listed in the
    /// destination. Trigger Mode for "fixed" Delivery Mode can be edge or level.
    Fixed = 0b000,
    /// Deliver the signal on the INTR signal of the processor core that is
    /// specified destination. Trigger Mode for "lowest priority". Delivery Mode
    /// executing at the lowest priority among all the processors listed in the
    /// can be edge or level.
    LowestPriority = 0b001,
    /// System Management Interrupt. A delivery mode equal to SMI requires an
    /// edge trigger mode. The vector information is ignored but must be
    /// programmed to all zeroes for future compatibility.
    Smi = 0b010,
    /// Deliver the signal on the NMI signal of all processor cores listed in the
    /// destination. Vector information is ignored. NMI is treated as an edge
    /// triggered interrupt, even if it is programmed as a level triggered interrupt.
    /// For proper operation, this redirection table entry must be programmed to
    /// “edge” triggered interrupt.
    Nmi = 0b100,
    /// Deliver the signal to all processor cores listed in the destination by
    /// asserting the INIT signal. All addressed local APICs will assume their
    /// INIT state. INIT is always treated as an edge triggered interrupt, even if
    /// programmed otherwise. For proper operation, this redirection table entry
    /// must be programmed to “edge” triggered interrupt.
    Init = 0b101,
    /// Deliver the signal to the INTR signal of all processor cores listed in the
    /// destination as an interrupt that originated in an externally connected
    /// (8259A-compatible) interrupt controller. The INTA cycle that corresponds
    /// to this ExtINT delivery is routed to the external controller that is expected
    /// to supply the vector. A Delivery Mode of "ExtINT" requires an edge
    /// trigger mode.
    ExtInt = 0b111,
    #[doc(hidden)]
    _Invalid1 = 0b011,
    #[doc(hidden)]
    _Invalid2 = 0b110,
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
