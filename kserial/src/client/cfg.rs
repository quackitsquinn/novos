use core::sync::atomic::AtomicBool;

use crate::common::macros::cfg_value;

cfg_value!(OUTPUT_SERIAL,AtomicBool,bool,true,set: set, get: should);

cfg_value!(
    PACKET_MODE,
    AtomicBool,
    bool,
    false,
    set: set,
    get: is
);
