use modular_bitfield::prelude::*;

use crate::mp::lapic::DeliverMode;

#[bitfield(bytes = 4)]
pub struct TimerLvt {
    pub vector: B8,
    #[skip]
    __: B4,
    pub delivery_status: bool,
    #[skip]
    __: B4,
    pub mask: bool,
    pub timer_mode: B2,
    #[skip]
    __: B12,
}
