//! Configuration options for the kserial client.
use core::sync::atomic::AtomicBool;

use crate::common::macros::cfg_value;

cfg_value!(
    OUTPUT_SERIAL,
    AtomicBool,
    bool,
    true,
    set: pub set, 
    get: pub should);

cfg_value!(
    INPUT_SERIAL,
    AtomicBool,
    bool,
    true,
    set: pub set,
    get: pub should
);

cfg_value!(
    PACKET_MODE,
    AtomicBool,
    bool,
    false,
    set: pub(crate) set,
    get: pub(crate) is
);
