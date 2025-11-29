//! Local Vector Table (LVT) entries for the LAPIC.

use bitfield::bitfield;

use crate::{
    interrupts::KernelInterrupt,
    mp::lapic::{Lapic, lvt::timer::ApicTimerLvt},
};

pub mod timer;

bitfield! {
    pub struct SimpleLvtEntryValue(u32);
    impl Debug;
    pub u8, vector, set_vector: 7, 0;
    pub bool, delivery_status, _: 12;
    pub bool, mask, set_mask: 16;
    pub u8, from LvtDeliverMode, delivery_mode, set_delivery_mode: 14, 13;
    bool, trigger_mode, set_trigger_mode: 15;
    bool, remote_irr, _: 14;
    bool, interrupt_input_pin_polarity, set_interrupt_input_pin_polarity: 13;

}

/// Simple Local Vector Table (LVT) entry interface.
// (lapic, byte_offset)
pub struct LvtEntry<'a> {
    lapic: &'a Lapic,
    reg_off: usize,
    delivery_mode_reserved: bool,
    is_lint: bool,
}

impl LvtEntry<'_> {
    /// Creates a new SimpleLvtEntry interface for the given LAPIC and byte offset.
    /// # Arguments
    /// * `lapic` - Reference to the LAPIC.
    /// * `byte_off` - Byte offset of the LVT entry register.
    /// * `delivery_mode_reserved` - Whether the delivery mode field is reserved for this LVT entry.
    pub(super) unsafe fn new(
        lapic: &Lapic,
        byte_off: usize,
        delivery_mode_reserved: bool,
    ) -> LvtEntry<'_> {
        LvtEntry {
            lapic,
            reg_off: byte_off,
            delivery_mode_reserved,
            is_lint: false,
        }
    }

    /// Marks this LVT entry as a LINT entry.
    /// This is used to enable special handling for LINT entries.
    pub(super) unsafe fn is_lint(mut self) -> Self {
        self.is_lint = true;
        self
    }

    /// Reads the LVT entry.
    pub fn read(&self) -> SimpleLvtEntryValue {
        let raw_value = unsafe { self.lapic.read_offset::<u32>(self.reg_off) };
        SimpleLvtEntryValue(raw_value)
    }

    /// Writes to the LVT entry.
    /// # Safety
    /// The caller must ensure that the given LVT entry value is valid and does not conflict with other LVT entries.
    pub unsafe fn write(&self, lvt: SimpleLvtEntryValue) {
        unsafe {
            self.lapic.write_offset::<u32>(self.reg_off, lvt.0);
        }
    }

    /// Updates the LVT entry by applying the given function to the current value.
    /// # Safety
    /// The caller must ensure that the modifications made by the function are valid.
    pub unsafe fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut SimpleLvtEntryValue),
    {
        let mut lvt = self.read();
        f(&mut lvt);
        unsafe {
            self.write(lvt);
        }
    }

    /// Sets whether the LVT entry is masked or unmasked.
    pub unsafe fn set_masked(&self, masked: bool) {
        unsafe {
            self.update(|lvt| {
                lvt.set_mask(masked);
            })
        };
    }

    /// Returns whether the LVT entry is masked.
    pub fn is_masked(&self) -> bool {
        self.read().mask()
    }

    /// Returns the delivery mode of the LVT entry.
    pub fn delivery_mode(&self) -> Option<LvtDeliverMode> {
        if self.delivery_mode_reserved {
            return None;
        }
        Some(self.read().delivery_mode().into())
    }

    /// Sets the delivery mode of the LVT entry.
    pub unsafe fn set_delivery_mode(&self, mode: LvtDeliverMode) -> Option<()> {
        if self.delivery_mode_reserved {
            return None;
        }
        unsafe {
            self.update(|lvt| {
                lvt.set_delivery_mode(mode.into());
            })
        };
        Some(())
    }

    /// Returns the interrupt vector of the LVT entry.
    pub fn vector(&self) -> u8 {
        self.read().vector()
    }

    /// Sets the interrupt vector of the LVT entry.
    pub fn set_vector(&self, vector: KernelInterrupt) {
        unsafe {
            self.update(|lvt| {
                lvt.set_vector(vector as u8);
            })
        };
    }

    /// If this LVT entry is a LINT entry, provides access to the Local Interrupt Pin interface.
    pub fn local_interrupt_pin(&self) -> Option<LocalInterruptPin<'_, '_>> {
        if !self.is_lint {
            return None;
        }
        Some(LocalInterruptPin::new(self))
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LvtDeliverMode {
    Fixed = 0b000,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    ExtInt = 0b111,
}

impl From<LvtDeliverMode> for u8 {
    fn from(value: LvtDeliverMode) -> Self {
        value as u8
    }
}

impl From<u8> for LvtDeliverMode {
    fn from(value: u8) -> Self {
        match value {
            0b000 => LvtDeliverMode::Fixed,
            0b010 => LvtDeliverMode::Smi,
            0b100 => LvtDeliverMode::Nmi,
            0b101 => LvtDeliverMode::Init,
            0b111 => LvtDeliverMode::ExtInt,
            _ => panic!("Invalid LVT delivery mode"),
        }
    }
}

pub struct LocalInterruptPin<'a, 'b> {
    lvt_entry: &'a LvtEntry<'b>,
}

impl LocalInterruptPin<'_, '_> {
    /// Creates a new LocalInterruptPin interface for the given LVT entry.
    pub(super) fn new<'a, 'b>(lvt_entry: &'a LvtEntry<'b>) -> LocalInterruptPin<'a, 'b> {
        LocalInterruptPin { lvt_entry }
    }

    /// Provides access to the underlying LVT entry.
    pub fn lvt_entry(&self) -> &LvtEntry<'_> {
        &self.lvt_entry
    }

    /// Sets the polarity of the LVT entry.
    pub fn set_polarity(&self, active_high: bool) {
        unsafe {
            self.lvt_entry.update(|lvt| {
                lvt.set_interrupt_input_pin_polarity(!active_high);
            })
        };
    }

    /// Sets the trigger mode of the LVT entry.
    pub fn set_trigger_mode(&self, edge_triggered: bool) {
        unsafe {
            self.lvt_entry.update(|lvt| {
                lvt.set_trigger_mode(!edge_triggered);
            })
        };
    }

    /// Returns true if the Remote IRR bit is set, false otherwise.
    pub fn remote_irr(&self) -> bool {
        self.lvt_entry.read().remote_irr()
    }

    /// Returns true if the interrupt is edge triggered, false if level triggered.
    pub fn is_edge_triggered(&self) -> bool {
        !self.lvt_entry.read().trigger_mode()
    }

    /// Returns true if the interrupt input pin is active high, false if active low.
    pub fn is_active_high(&self) -> bool {
        !self.lvt_entry.read().interrupt_input_pin_polarity()
    }
}

/// Corrected Machine Check Interrupt (CMCI) LVT entry offset.
pub const LVT_CMCI_OFFSET: usize = 0x2F0;
/// LINT0 LVT entry offset.
pub const LVT_LINT0_OFFSET: usize = 0x350;
/// LINT1 LVT entry offset.
pub const LVT_LINT1_OFFSET: usize = 0x360;
/// Error LVT entry offset.
pub const LVT_ERROR_OFFSET: usize = 0x370;
/// Performance Monitoring Counters LVT entry offset.
pub const LVT_PREFORM_MON_COUNTERS_OFFSET: usize = 0x340;
/// Thermal Sensor LVT entry offset.
pub const LVT_THERMAL_SENSOR_OFFSET: usize = 0x330;

pub struct LocalVectorTable<'a>(&'a Lapic);

impl LocalVectorTable<'_> {
    /// Creates a new LocalVectorTable interface for the given LAPIC.
    pub(super) fn new(lapic: &Lapic) -> LocalVectorTable<'_> {
        LocalVectorTable(lapic)
    }

    /// Provides access to the Corrected Machine Check Interrupt (CMCI) LVT entry.
    pub fn corrected_machine_check(&self) -> LvtEntry<'_> {
        unsafe { LvtEntry::new(self.0, LVT_CMCI_OFFSET, false) }
    }

    /// Provides access to the Timer LVT entry.
    pub fn timer(&self) -> ApicTimerLvt<'_> {
        ApicTimerLvt::new(self.0)
    }

    /// Provides access to the LINT0 LVT entry.
    pub fn lint0(&self) -> LvtEntry<'_> {
        unsafe { LvtEntry::new(self.0, LVT_LINT0_OFFSET, true).is_lint() }
    }

    /// Provides access to the LINT1 LVT entry.
    pub fn lint1(&self) -> LvtEntry<'_> {
        unsafe { LvtEntry::new(self.0, LVT_LINT1_OFFSET, true).is_lint() }
    }

    /// Provides access to the Error LVT entry.
    ///
    /// Delivery mode is reserved for this entry.
    pub fn error(&self) -> LvtEntry<'_> {
        unsafe { LvtEntry::new(self.0, LVT_ERROR_OFFSET, true) }
    }

    /// Provides access to the Performance Monitoring Counters LVT entry.
    pub fn performance_monitoring_counters(&self) -> LvtEntry<'_> {
        unsafe { LvtEntry::new(self.0, LVT_PREFORM_MON_COUNTERS_OFFSET, false) }
    }

    /// Provides access to the Thermal Sensor LVT entry.
    pub fn thermal_sensor(&self) -> LvtEntry<'_> {
        unsafe { LvtEntry::new(self.0, LVT_THERMAL_SENSOR_OFFSET, false) }
    }
}
