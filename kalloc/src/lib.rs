#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(pointer_is_aligned_to)]
#![warn(missing_debug_implementations)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::sync::atomic::AtomicBool;

extern crate alloc;

#[macro_use]
#[allow(unused_macros)]
pub(crate) mod log;

pub(crate) mod alloc_wrap;
pub mod block_alloc;
pub mod locked_vec;
pub mod mut_alloc;

pub use alloc_wrap::GlobalAllocatorWrapper;

pub(crate) const ALLOC_LOG: AtomicBool = AtomicBool::new(false);

pub fn enable_logging() {
    ALLOC_LOG.store(true, core::sync::atomic::Ordering::Relaxed);
}

pub fn disable_logging() {
    ALLOC_LOG.store(false, core::sync::atomic::Ordering::Relaxed);
}
