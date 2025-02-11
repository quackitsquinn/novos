use core::convert::Infallible;

use crate::{declare_module, util::OnceMutex};

pub mod range;
pub mod virt_alloc;

pub use range::VirtualAddressRange;
use virt_alloc::VirtualAddressMapper;
use x86_64::VirtAddr;

pub static VIRT_MAPPER: OnceMutex<VirtualAddressMapper> = OnceMutex::new();

// Upper half of the 48-bit address space + 268mb of virtual memory for the kernel.
// We have access to 1.8TB of virtual memory
const VIRT_MAP_START: VirtAddr = VirtAddr::new_truncate(0xffff_9000_0000);
const MAX_VIRT_ADDRESS: VirtAddr = VirtAddr::new_truncate(0xffff_ffff_ffff);

declare_module!("virtual memory", init);

fn init() -> Result<(), Infallible> {
    VIRT_MAPPER.init(unsafe { VirtualAddressMapper::new(VIRT_MAP_START, MAX_VIRT_ADDRESS) });
    Ok(())
}
