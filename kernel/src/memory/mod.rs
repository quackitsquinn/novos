use core::{convert::Infallible, mem};

use cake::{ResourceGuard, log::info};
use nmm::{
    InitConfig, MapFlags, MapSource,
    arch::HIGHER_HALF_START,
    paging::{Address, AddressExt, PageTable, PageTableEntry, PageTableIndex, VirtAddr},
};
use x86_64::{
    VirtAddr as XVirtAddr,
    registers::control::Cr3,
    structures::paging::{Page, PageTableFlags, page::PageRangeInclusive},
};

use crate::{
    declare_module,
    memory::paging::{KernelPageSize, map::map},
    requests::{KERNEL_ELF, MEMORY_MAP, PHYSICAL_MEMORY_OFFSET},
};

pub mod allocator;
pub mod elf_req_data;
pub mod paging;
pub mod req_data;

/// Enables or disables allocation debugging based on the ALLOC_DEBUG environment variable.
pub const ALLOC_DEBUG: bool = option_env!("ALLOC_DEBUG").is_some();

declare_module!("memory", init);

fn init() -> Result<(), Infallible> {
    let hhdm_offset = *PHYSICAL_MEMORY_OFFSET
        .get()
        .expect("Physical memory offset not provided by bootloader");
    let memory_map = MEMORY_MAP.lock_limine();
    let memory_map = memory_map.entries();
    let l4_phys = nmm::arch::pml4_phys();
    let pml4_vaddr = l4_phys
        .translate_offset(hhdm_offset)
        .expect("Failed to translate PML4 physical address to virtual address");
    let mut root = unsafe { &mut *(pml4_vaddr.as_mut_ptr::<PageTable>()) };
    let mut recursive_idx = None;
    let max = PageTableIndex::MAX.value();
    for i in
        PageTableIndex::iter_range(PageTableIndex::new(max - (max / 4))..PageTableIndex::MAX).rev()
    {
        if root.read_entry(i).is_present() {
            continue;
        }

        unsafe { root.set_entry(i, PageTableEntry::new(l4_phys, MapFlags::WRITABLE)) };

        recursive_idx = Some(i);
        break;
    }

    let init = InitConfig::find_scratch_page(
        recursive_idx.expect("No free recursive slot found in PML4"),
        map::nmm_managed_range::RANGE,
        unsafe { mem::transmute(memory_map) },
    )
    .unwrap();
    info!("Initializing nmm {:?}", init);
    unsafe { nmm::init(init) }.expect("Failed to initialize memory manager");
    info!("Memory manager initialized");
    Ok(())
}
