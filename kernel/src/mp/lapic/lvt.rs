use modular_bitfield::prelude::*;

use crate::mp::id;

/// Local Vector Table (LVT) entry for the timer.
#[bitfield(bytes = 4)]
pub struct TimerLvt {
    /// Interrupt vector.
    pub vector: B8,
    #[skip]
    __: B4,
    /// Delivery status. Must not be written by software.
    pub delivery_status: bool,
    #[skip]
    __: B4,
    /// Mask the timer interrupt.
    pub mask: bool,
    /// The mode of the timer.
    pub timer_mode: B2,
    #[skip]
    __: B12,
}

id!(TimerLvt, REGISTER, 0x320);
