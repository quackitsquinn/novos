mod cfg;
mod gdb;
mod packet;
mod qemu_ctl;

use std::sync::Arc;

pub use cfg::Config;

pub trait HasInteriorMut {}

impl<T> HasInteriorMut for Arc<T> {}
