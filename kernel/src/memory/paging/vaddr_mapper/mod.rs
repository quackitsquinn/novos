//! Virtual address mapping utilities. This includes a simple virtual address allocator.
//! This is used to allocate virtual address space for mapping physical memory.
use core::convert::Infallible;

use crate::declare_module;

pub mod range;
pub mod virt_alloc;

use cake::OnceMutex;
pub use range::VirtualAddressRange;
use virt_alloc::VirtualAddressMapper;

use super::map;

/// The global virtual address mapper.
pub static VIRT_MAPPER: OnceMutex<VirtualAddressMapper> = OnceMutex::uninitialized();

declare_module!("virtual memory", init);

fn init() -> Result<(), Infallible> {
    VIRT_MAPPER.call_init(|| unsafe {
        VirtualAddressMapper::new(map::KERNEL_PHYS_MAP_START, map::KERNEL_PHYS_MAP_END)
    });
    Ok(())
}
