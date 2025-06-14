use core::convert::Infallible;

use crate::{
    declare_module,
    util::OnceMutex,
};

pub mod range;
pub mod virt_alloc;

pub use range::VirtualAddressRange;
use virt_alloc::VirtualAddressMapper;

use super::map;

pub static VIRT_MAPPER: OnceMutex<VirtualAddressMapper> = OnceMutex::uninitialized();

declare_module!("virtual memory", init);

fn init() -> Result<(), Infallible> {
    VIRT_MAPPER.init(unsafe {
        VirtualAddressMapper::new(map::KERNEL_PHYS_MAP_START, map::KERNEL_PHYS_MAP_END)
    });
    Ok(())
}
