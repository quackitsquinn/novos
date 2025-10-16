//! Kalloc - A Kernel Memory Allocator
#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(pointer_is_aligned_to)]
#![warn(missing_debug_implementations)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::sync::atomic::AtomicBool;

extern crate alloc;

#[macro_use]
#[allow(unused_macros)]
pub(crate) mod alog;

pub(crate) mod alloc_wrap;
pub mod block_alloc;
pub mod locked_vec;
pub mod mut_alloc;

pub use alloc_wrap::GlobalAllocatorWrapper;

pub(crate) static ALLOC_LOG: AtomicBool = AtomicBool::new(false);
/// pub(crate) static FRAME_OUTPUT_FN: Once<fn(&[u8])> = Once::new();

// pub(crate) fn frame_output(data: &[u8]) {
//     if let Some(fn_ptr) = FRAME_OUTPUT_FN.get() {
//         adebug!("Logging frame output with length: {}", data.len());
//         fn_ptr(data);
//     }
// }

// pub fn set_frame_output_fn(fn_ptr: fn(&[u8])) -> bool {
//     if FRAME_OUTPUT_FN.is_completed() {
//         return false;
//     }
//     FRAME_OUTPUT_FN.call_once(|| fn_ptr);
//     true
// }

/// Enables logging for the allocator.
pub fn enable_logging() {
    ALLOC_LOG.store(true, core::sync::atomic::Ordering::Relaxed);
}

/// Disables logging for the allocator
pub fn disable_logging() {
    ALLOC_LOG.store(false, core::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn should_log() -> bool {
    ALLOC_LOG.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) mod test_common {
    use core::{alloc::Layout, ptr::NonNull};
    use std::alloc::{Allocator, Global};

    /// A wrapper around an allocated memory region that will be deallocated when it goes out of scope.
    pub struct DeferDealloc {
        pub(crate) layout: Layout,
        pub(crate) ptr: NonNull<[u8]>,
    }

    impl DeferDealloc {
        pub fn alloc(layout: Layout) -> (Self, NonNull<[u8]>) {
            let ptr = Global.allocate(layout).expect("Failed to allocate");
            (Self { layout, ptr }, ptr)
        }
    }

    impl Drop for DeferDealloc {
        fn drop(&mut self) {
            unsafe {
                Global.deallocate(self.ptr.cast(), self.layout);
            }
        }
    }
}
