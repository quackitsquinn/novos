//! Local APIC Timer LVT entry and related enums.
use core::fmt::Debug;

use bitfield::bitfield;

use crate::mp::id;

bitfield! {
    /// Local Vector Table (LVT) entry for the timer.
    pub struct ApicTimerLvt(u32);
    impl Debug;
    /// The interrupt vector number.
    pub u8, vector, set_vector: 7, 0;
    /// The delivery mode of the interrupt.
    pub bool, delivery_status, _: 12;
    /// The mask bit (0 = enabled, 1 = masked).
    /// When masked, the interrupt will not be delivered.
    pub bool, mask, set_mask: 16;
    /// The timer mode (0 = one-shot, 1 = periodic).
    pub u8, from TimerMode, timer_mode, set_timer_mode: 18, 17;
}

id!(ApicTimerLvt, REGISTER, 0x320);

/// Timer mode for the LAPIC timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TimerMode {
    /// One-shot mode. The timer counts down once and then stops.
    OneShot = 0b00,
    /// Periodic mode. The timer reloads the initial count value and continues counting down.
    Periodic = 0b01,
    /// TSC-Deadline mode. The timer is triggered when the TSC reaches the value in the MSR.
    TscDeadline = 0b10,
}

impl From<TimerMode> for u8 {
    fn from(value: TimerMode) -> Self {
        value as u8
    }
}

/// Timer divider values for the LAPIC timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TimerDivider {
    /// Divide by 1.
    By1 = 0b111,
    /// Divide by 2.
    By2 = 0b0000,
    /// Divide by 4.
    By4 = 0b0001,
    /// Divide by 8.
    By8 = 0b010,
    /// Divide by 16.
    By16 = 0b011,
    /// Divide by 32.
    By32 = 0b100,
    /// Divide by 64.
    By64 = 0b101,
    /// Divide by 128.
    By128 = 0b110,
}
