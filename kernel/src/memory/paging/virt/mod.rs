use crate::util::OnceMutex;

pub mod range;
pub mod virt_alloc;

static VIRT_MAPPER: OnceMutex<VirtualAddressMapper> = OnceMutex::new();

pub fn init() {
    // Do nothing for now
}

use alloc::vec::Vec;
pub use range::VirtualAddressRange;
use virt_alloc::VirtualAddressMapper;
use x86_64::structures::paging::PageTable;

fn find_used_virtual_ranges(pgtbl: &PageTable) -> Vec<VirtualAddressRange> {
    todo!()
}
