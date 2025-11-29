
use bitfield::bitfield;

use crate::mp::id;

bitfield! {
    /// LAPIC Version Register.
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct LapicVersion(u32);
    impl Debug;
    /// The version number of the LAPIC.
    pub u8, version, _: 7, 0;
    /// The maximum LVT entry supported (number of LVT entries - 1).
    pub u8, max_lvt_entry, _: 23, 16;
    /// Indicates whether the LAPIC supports EOI broadcast suppression.
    /// 0 = does not support, 1 = supports.
    pub bool, supports_eoi_broadcast_suppression, _: 24;
}

id!(LapicVersion, REGISTER, 0x30);
