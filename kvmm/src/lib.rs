#![no_std]
#![no_main]

use x86_64::structures::paging::{Page, PhysFrame, Size4KiB};

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod phys;
#[cfg(feature = "alloc")]
pub mod virt;

pub type KernelPageSize = Size4KiB;
pub type KernelPage = Page<KernelPageSize>;
pub type KernelPhysFrame = PhysFrame<KernelPageSize>;
