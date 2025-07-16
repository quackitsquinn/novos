#![cfg_attr(not(test), no_std)]

use x86_64::structures::paging::{Page, PhysFrame, Size4KiB};

#[cfg(any(feature = "alloc", test))]
extern crate alloc;

pub mod phys;
#[cfg(any(feature = "alloc", test))]
pub mod virt;

pub type KernelPageSize = Size4KiB;
pub type KernelPage = Page<KernelPageSize>;
pub type KernelPhysFrame = PhysFrame<KernelPageSize>;
