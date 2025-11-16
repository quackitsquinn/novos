use bitfield::bitfield;

use crate::mp::id;

bitfield! {
    /// Spurious Interrupt Vector Register (SVR).
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SpuriousInterruptVector(u32);
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

id!(SpuriousInterruptVector, REGISTER, 0xF0);
