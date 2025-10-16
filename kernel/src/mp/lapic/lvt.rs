use core::fmt::Debug;

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

impl Debug for TimerLvt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TimerLvt")
            .field("vector", &self.vector())
            .field("delivery_status", &self.delivery_status())
            .field("mask", &self.mask())
            .field("timer_mode", &self.timer_mode())
            .finish()
    }
}
