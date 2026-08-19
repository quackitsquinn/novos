use core::{convert::Infallible, mem};

use cake::{ResourceGuard, log::info};
use nmm::{
    InitConfig, MapFlags, MapSource,
    arch::HIGHER_HALF_START,
    paging::{Address, AddressExt, PageTable, VirtAddr},
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
    let init = InitConfig::find_scratch_page(
        VirtAddr::new_truncate(hhdm_offset),
        map::nmm_managed_range::RANGE,
        unsafe { mem::transmute(memory_map) },
    )
    .unwrap();
    info!("Initializing nmm {:?}", init);
    unsafe { nmm::init(init) }.expect("Failed to initialize memory manager");
    info!("Memory manager initialized");
    Ok(())
}
