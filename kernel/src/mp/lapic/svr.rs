use bitfield::bitfield;

use crate::{
    interrupts::KernelInterrupt,
    mp::{id, lapic::Lapic},
};

bitfield! {
    /// Spurious Interrupt Vector Register (SVR).
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SpuriousInterruptVectorValue(u32);
    impl Debug;
    /// The spurious interrupt vector number.
    pub u8, vector, set_vector: 7, 0;
    /// The APIC software enable/disable bit.
    /// 0 = disabled, 1 = enabled.
    pub bool, apic_enable, set_apic_enable: 8;
    /// The focus processor checking bit.
    /// 0 = disabled, 1 = enabled.
    pub bool, focus_processor_checking, set_focus_processor_checking: 9;
    /// The EOI broadcast suppression bit.
    /// 0 = disabled, 1 = enabled.
    pub bool, eoi_broadcast_suppression, set_eoi_broadcast_suppression: 12;
}

id!(SpuriousInterruptVectorValue, REGISTER, 0xF0);

pub struct SpuriousInterruptVector<'a>(&'a Lapic);

impl SpuriousInterruptVector<'_> {
    /// Creates a new SpuriousInterruptVector interface for the given LAPIC.
    pub(super) fn new(lapic: &Lapic) -> SpuriousInterruptVector<'_> {
        SpuriousInterruptVector(lapic)
    }

    /// Reads the Spurious Interrupt Vector Register (SVR).
    pub fn read(&self) -> SpuriousInterruptVectorValue {
        SpuriousInterruptVectorValue(unsafe {
            self.0
                .read_offset::<u32>(SpuriousInterruptVectorValue::REGISTER)
        })
    }

    /// Writes to the Spurious Interrupt Vector Register (SVR).
    ///
    /// # Safety
    /// The caller must also ensure that the value being written is valid.
    pub unsafe fn write(&self, svr: SpuriousInterruptVectorValue) {
        unsafe {
            self.0
                .write_offset::<u32>(SpuriousInterruptVectorValue::REGISTER, svr.0);
        }
    }

    /// Enables the LAPIC by setting the APIC enable bit in the SVR and setting the spurious interrupt vector.
    pub fn enable(&self) {
        unsafe {
            let mut svr = SpuriousInterruptVectorValue(0);

            svr.set_vector(KernelInterrupt::Spurious as u8); // Set to some default vector
            svr.set_apic_enable(true);

            self.write(svr);
        }
    }
}
