use core::convert::Infallible;

use crate::{declare_module, util::OnceMutex};

pub mod range;
pub mod virt_alloc;

use alloc::vec;
pub use range::VirtualAddressRange;
use virt_alloc::VirtualAddressMapper;
use x86_64::VirtAddr;


pub static VIRT_MAPPER: OnceMutex<VirtualAddressMapper> = OnceMutex::new();
//
const VIRT_MAP_START: u64 = 0x100000000000;

declare_module!("virtual memory", init);

fn init() -> Result<(), Infallible> {
    // TODO: Proper virtual address space mapping even though its probably not needed because
    // i can't imagine a scenario where we need more than 105TB of virtual memory
    VIRT_MAPPER.init(unsafe {
        VirtualAddressMapper::from_unused_ranges(vec![VirtualAddressRange::new(
            VirtAddr::new(VIRT_MAP_START),
            (1 << 48) - VIRT_MAP_START,
        )])
    });
    Ok(())
}
