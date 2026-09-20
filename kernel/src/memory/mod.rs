use core::{convert::Infallible, mem, ptr::addr_of};

use crate::{
    declare_module,
    requests::{MEMORY_MAP, PHYSICAL_MEMORY_OFFSET},
};
use cake::log::info;
use nmm::{
    InitConfig, MapFlags,
    paging::{
        Address, AddressExt, MemoryRange, PageTable, PageTableEntry, PageTableIndex, VirtAddr,
    },
};

pub mod allocator;
pub mod elf_req_data;
pub mod req_data;

/// Enables or disables allocation debugging based on the ALLOC_DEBUG environment variable.
pub const ALLOC_DEBUG: bool = option_env!("ALLOC_DEBUG").is_some();

unsafe extern "C" {
    #[link_name = "kernel_start"]
    static KERNEL_START: u8;
    #[link_name = "kernel_end"]
    static KERNEL_END: u8;
}

/// Returns the memory range occupied by the kernel binary in virtual memory.
pub fn kernel_range() -> MemoryRange<VirtAddr> {
    let start = unsafe { Address::new(&KERNEL_START as *const u8 as u64) };
    let end = unsafe { Address::new(&KERNEL_END as *const u8 as u64) };
    MemoryRange::new(start, end)
}

declare_module!("memory", init);

fn init() -> Result<(), Infallible> {
    unsafe { init_nmm() };
    Ok(())
}

unsafe fn init_nmm() {
    let memory_map = MEMORY_MAP.lock_limine();
    let memory_map = memory_map.entries();
    let recursive_idx = unsafe { find_set_recursive_entry() };

    let init = InitConfig::find_scratch_page(
        recursive_idx.expect("No free recursive slot found in PML4"),
        map::nmm_managed_range::RANGE,
        unsafe { mem::transmute(memory_map) },
    )
    .unwrap();

    info!("Initializing nmm {:?}", init);
    unsafe { nmm::init(init) }.expect("Failed to initialize memory manager");
    info!("Memory manager initialized");
}

const RECURSIVE_RANGE_START: PageTableIndex =
    PageTableIndex::new(PageTableIndex::MAX.0 - (PageTableIndex::MAX.0 / 4));
const RECURSIVE_RANGE_END: PageTableIndex =
    unsafe { PageTableIndex::new_unchecked(PageTableIndex::MAX.0 - 1) };

unsafe fn find_set_recursive_entry() -> Option<PageTableIndex> {
    let l4_phys = nmm::arch::pml4_phys();
    let pml4_vaddr = l4_phys
        .translate_offset(*PHYSICAL_MEMORY_OFFSET.get().unwrap())
        .expect("Failed to translate PML4 physical address to virtual address");
    let root = unsafe { &mut *(pml4_vaddr.as_mut_ptr::<PageTable>()) };
    for i in PageTableIndex::iter_range(RECURSIVE_RANGE_START..RECURSIVE_RANGE_END).rev() {
        if root.read_entry(i).is_present() {
            continue;
        }
        unsafe {
            root.set_entry(i, PageTableEntry::new(l4_phys, MapFlags::WRITABLE));
        }
        return Some(i);
    }
    None
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
use nmm::kernel_map;

kernel_map! {
    . = (higher_half + 512 GiB),
    NMM_MANAGED_RANGE = 2 GiB; align 1 GiB,
    NMM_ZERO_PAGE = Large; align Large,
    KERNEL_HEAP = 16 MiB; align 2 MiB,
    KERNEL_PHYS_MAP = 256 MiB; align 2 MiB,
    KERNEL_REMAP = 256 MiB; align 2 MiB,
    FRAMEBUFFER = 2 MiB; align 2 MiB,
    ADDRESS_SPACE_INFO = 4 KiB; align 4 KiB,
}
