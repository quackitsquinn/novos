//! Local APIC Timer LVT entry and related enums.
use core::fmt::Debug;

use bitfield::bitfield;

use crate::mp::id;

bitfield! {
    /// Local Vector Table (LVT) entry for the timer.
    pub struct LocalTimerLvtValue(u32);
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

id!(LocalTimerLvtValue, REGISTER, 0x320);

/// The offset for the Timer Divide Configuration register of the LAPIC timer.
pub const TIMER_DIVIDE_OFFSET: usize = 0x3E0;
/// The offset for the Initial Count register of the LAPIC timer.
pub const TIMER_INITIAL_COUNT_OFFSET: usize = 0x380;
/// The offset for the Current Count register of the LAPIC timer.
pub const TIMER_CURRENT_COUNT_OFFSET: usize = 0x390;

pub struct ApicTimerLvt<'a>(&'a crate::mp::lapic::Lapic);

impl ApicTimerLvt<'_> {
    /// Creates a new ApicTimerLvt interface for the given LAPIC.
    pub(super) fn new(lapic: &crate::mp::lapic::Lapic) -> ApicTimerLvt<'_> {
        ApicTimerLvt(lapic)
    }

    /// Reads the Local Timer LVT entry.
    pub fn read(&self) -> LocalTimerLvtValue {
        let raw_value = unsafe { self.0.read_offset::<u32>(LocalTimerLvtValue::REGISTER) };
        LocalTimerLvtValue(raw_value)
    }

    /// Writes to the Local Timer LVT entry.
    /// # Safety
    /// The caller must ensure that the given LVT entry value is valid.
    pub unsafe fn write(&self, lvt: LocalTimerLvtValue) {
        unsafe {
            self.0
                .write_offset::<u32>(LocalTimerLvtValue::REGISTER, lvt.0);
        }
    }

    /// Updates the Local Timer LVT entry by applying the given function to the current value.
    /// # Safety
    /// The caller must ensure that the modifications made by the function are valid.
    pub unsafe fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut LocalTimerLvtValue),
    {
        let mut lvt = self.read();
        f(&mut lvt);
        unsafe {
            self.write(lvt);
        }
    }

    /// Sets the LAPIC timer divider.
    pub fn set_timer_divider(&self, divider: TimerDivider) {
        let val = divider as u8;
        // For some reason, there is a reserved bit in the middle of the divider value. Intel loves making things complicated.
        let up_bit = (val << 1) & 0b1000;
        let low_bits = val & 0b11;
        let final_val = up_bit | low_bits;
        unsafe {
            self.0
                .write_offset::<u32>(TIMER_DIVIDE_OFFSET, final_val as u32);
        }
    }

    /// Reads the current count value of the LAPIC timer.
    pub fn read_timer_current_count(&self) -> u32 {
        unsafe { self.0.read_offset::<u32>(TIMER_CURRENT_COUNT_OFFSET) }
    }

    /// Writes the initial count value to the LAPIC timer.
    pub fn write_timer_initial_count(&self, count: u32) {
        unsafe {
            self.0
                .write_offset::<u32>(TIMER_INITIAL_COUNT_OFFSET, count);
        }
    }

    /// Attempts to read the base frequency of the LAPIC timer in Hz.
    /// Returns `None` if the frequency cannot be determined.
    pub fn timer_base_freq_hz(&self) -> Option<u64> {
        raw_cpuid::CpuId::with_cpuid_reader(raw_cpuid::CpuIdReaderNative)
            .get_tsc_info()
            .map_or(None, |tsc_info| tsc_info.tsc_frequency())
    }

    /// Checks if the LAPIC timer is of consistent speed (i.e., not affected by power-saving modes).
    /// Returns `true` if the timer is consistent speed, `false` otherwise.
    #[doc(alias = "arat")]
    pub fn timer_is_consistent_speed(&self) -> bool {
        raw_cpuid::CpuId::with_cpuid_reader(raw_cpuid::CpuIdReaderNative)
            .get_thermal_power_info()
            .map_or(false, |therm_info| therm_info.has_arat())
    }
}

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
