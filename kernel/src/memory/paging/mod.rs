//! Abstractions for managing memory paging in the kernel.

use core::convert::Infallible;

use cake::OnceRwLock;
use x86_64::{
    VirtAddr,
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB},
};

use crate::{declare_module, requests::PHYSICAL_MEMORY_OFFSET};

pub use self::page_table::ActivePageTable;

pub mod addr_space;
mod builder;
pub mod kernel;
mod page_table;
pub mod page_tree;
pub mod phys;
pub mod vaddr_mapper;

/// The size of a kernel page.
pub type KernelPageSize = Size4KiB;
/// A kernel page.
pub type KernelPage = Page<KernelPageSize>;
/// A kernel phys frame
pub type KernelPhysFrame = PhysFrame<KernelPageSize>;

/// The active kernel page table.
pub static ACTIVE_PAGE_TABLE: OnceRwLock<ActivePageTable> = OnceRwLock::new();

declare_module!("paging", init);

fn init() -> Result<(), Infallible> {
    let cr3 = Cr3::read();
    let off = *PHYSICAL_MEMORY_OFFSET
        .get()
        .expect("physical memory offset uninitialized");
    let page_table = unsafe { &mut *((cr3.0.start_address().as_u64() + off) as *mut PageTable) };
    let offset_table = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(off)) };
    ACTIVE_PAGE_TABLE.init(|| ActivePageTable::new(offset_table));

    phys::MODULE.init();
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
    use x86_64::VirtAddr;

    macro_rules! define_map {
        ($name:ident, $start:expr, $size:expr) => {
            const _: () = {
                assert!($start % 0x1000 == 0);
                assert!($size % 0x1000 == 0);
                assert!($start < MAX_VIRT_ADDR.as_u64());
                assert!($size < MAX_VIRT_ADDR.as_u64());
                assert!($start + $size < MAX_VIRT_ADDR.as_u64());
                ()
            };
            paste::paste! {
                #[doc = concat!("The raw start address of the ", stringify!($name), " memory region.")]
                pub const [<$name _RAW>]: u64 = $start;

                #[doc = concat!("The start address of the ", stringify!($name), " memory region.")]
                pub const [<$name _START>]: ::x86_64::VirtAddr = ::x86_64::VirtAddr::new_truncate([<$name _RAW>]);
                #[doc = concat!("The size of the ", stringify!($name), " memory region.")]
                pub const [<$name _SIZE>]: u64 = $size;
                #[doc = concat!("The raw end address of the ", stringify!($name), " memory region.")]
                pub const [<$name _END_RAW>]: u64 = $start + $size;
                #[doc = concat!("The end address of the ", stringify!($name), " memory region.")]
                pub const [<$name _END>]: ::x86_64::VirtAddr = ::x86_64::VirtAddr::new_truncate([<$name _END_RAW>]);
                #[doc = concat!("The start page of the ", stringify!($name), " memory region.")]
                pub const [<$name _START_PAGE>]: super::KernelPage = super::KernelPage::containing_address([<$name _START>]);
                #[doc = concat!("The end page of the ", stringify!($name), " memory region.")]
                pub const [<$name _END_PAGE>]: super::KernelPage = super::KernelPage::containing_address([<$name _END>]);
                #[doc = concat!("The page range of the ", stringify!($name), " memory region.")]
                pub const [<$name _PAGE_RANGE>]: ::x86_64::structures::paging::page::PageRangeInclusive<super::KernelPageSize> =
                    super::KernelPage::range_inclusive([<$name _START_PAGE>], [<$name _END_PAGE>]);
            }
        };
    }

    /// The start address of the kernel binary.
    /// Taken from linker script
    pub const KERNEL_BINARY: u64 = 0xFFFF_FFFF_8000_0000;
    /// The start of the higher half of the kernel.
    pub const HIGHER_HALF_START: u64 = 0xFFFF_8000_0000;
    /// The maximum virtual address.
    pub const MAX_VIRT_ADDR: VirtAddr = VirtAddr::new_truncate(u64::MAX);

    /// The start of the kernel.
    pub const KERNEL_START: u64 = HIGHER_HALF_START + 0x1000_0000;

    define_map!(KERNEL_HEAP, KERNEL_START, 0x100_0000); // 16MB

    define_map!(KERNEL_PHYS_MAP, KERNEL_HEAP_END_RAW, 0x1000_0000); // 256MB

    // Area used to remap the kernel onto a new page table. This area will not be used after the pml4 switch
    define_map!(KERNEL_REMAP, KERNEL_PHYS_MAP_END_RAW, 0x1000_0000); // 256MB
    // Where the framebuffer is mapped in the remapped kernel.
    define_map!(FRAMEBUFFER, KERNEL_PHYS_MAP_END_RAW, 0x1000_0000); // 2MB

    // The address space info structure mapped area.
    define_map!(ADDRESS_SPACE_INFO, FRAMEBUFFER_END_RAW, 0x1000); // 4KB
}
