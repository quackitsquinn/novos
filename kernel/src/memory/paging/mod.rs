//! Abstractions for managing memory paging in the kernel.

use core::convert::Infallible;

use cake::OnceRwLock;
use x86_64::{
    VirtAddr,
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB},
};

use crate::{declare_module, requests::PHYSICAL_MEMORY_OFFSET};

//mod builder;
// pub mod kernel;
//mod page_table;
//pub mod page_tree;

/// The size of a kernel page.
pub type KernelPageSize = Size4KiB;
/// A kernel page.
pub type KernelPage = Page<KernelPageSize>;
/// A kernel phys frame
pub type KernelPhysFrame = PhysFrame<KernelPageSize>;

declare_module!("paging", init);

fn init() -> Result<(), Infallible> {
    let cr3 = Cr3::read();
    let off = *PHYSICAL_MEMORY_OFFSET
        .get()
        .expect("physical memory offset uninitialized");
    let page_table = unsafe { &mut *((cr3.0.start_address().as_u64() + off) as *mut PageTable) };
    let offset_table = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(off)) };

    Ok(())
}

/// Defines various memory map constants used by the kernel.
///
/// KERNEL_* = Kernel memory
///
/// KERNEL_HEAP_* = Kernel heap memory
///
/// KERNEL_PHYS_MAP_* = Kernel misc memory (e.g virtual/physical memory mapping)
///
/// KERNEL_BINARY = Kernel binary memory
///
/// HIGHER_HALF_START = Start of the higher half of the kernel memory
pub mod map {
    use nmm::kernel_map;

    kernel_map! {
        . = (higher_half + 512 GiB),
        NMM_MANAGED_RANGE = 8 GiB; align 1 GiB,
        KERNEL_HEAP = 16 MiB; align 2 MiB,
        KERNEL_PHYS_MAP = 256 MiB; align 2 MiB,
        KERNEL_REMAP = 256 MiB; align 2 MiB,
        FRAMEBUFFER = 2 MiB; align 2 MiB,
        ADDRESS_SPACE_INFO = 4 KiB; align 4 KiB,
    }
}
