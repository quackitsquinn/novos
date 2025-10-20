//! InterProcessor Interrupts (IPIs) for Multi-Processor support.

use crate::{
    interrupts::KernelInterrupt,
    mp::{
        LAPIC,
        lapic::{DeliverMode, DestinationShorthand, InterruptCommandRegister},
    },
};

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

pub unsafe fn send_ipi(dest: IPIDestination, vector: KernelInterrupt) {
    let mut icr = InterruptCommandRegister::new()
        .with_vector(vector as u8)
        .with_delivery_mode(DeliverMode::Fixed)
        .with_destination_mode(false) // Physical mode
        .with_level(true) // Assert
        .with_trigger_mode(false); // Edge triggered

    /// Set the destination based on the specified type.
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
    unsafe { LAPIC.write_icr(icr) };
}
