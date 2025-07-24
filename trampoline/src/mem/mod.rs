use cake::{Mutex, Once, debug, info};
use kvmm::phys::{PhysAddrRange, frame_mapper::FrameMapper};
use limine::memory_map::{Entry, EntryType};

use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
};

mod kernel;

pub(crate) use kernel::Kernel;

use crate::requests::MEMORY_MAP;

pub struct UsableRangeIterator {
    map: &'static [&'static Entry],
    i: usize,
}

impl UsableRangeIterator {
    pub fn new() -> Self {
        UsableRangeIterator {
            map: MEMORY_MAP
                .get()
                .expect("Memory map request failed")
                .entries(),
            i: 0,
        }
    }
}

impl Iterator for UsableRangeIterator {
    type Item = PhysAddrRange;

    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.map.len() {
            let entry = self.map[self.i];
            self.i += 1;
            if entry.entry_type == EntryType::USABLE {
                return Some(PhysAddrRange::new(
                    PhysAddr::new(entry.base),
                    PhysAddr::new(entry.base + entry.length - 1),
                ));
            }
        }
        None
    }
}

pub(crate) static MAPPER: Once<Mutex<FrameMapper<UsableRangeIterator>>> = Once::new();
pub(crate) static PAGETABLE: Once<Mutex<OffsetPageTable>> = Once::new();

pub fn init() -> Kernel {
    info!("Initializing frame mapper and offset page table...");
    MAPPER.call_once(|| {
        let iter = UsableRangeIterator::new();
        debug!(
            "Usable range iterator created with {} entries",
            iter.map.len()
        );
        let mapper = FrameMapper::new(iter);
        Mutex::new(mapper)
    });
    info!("Frame mapper initialized!");
    PAGETABLE.call_once(|| {
        let pmo = *crate::requests::PHYSICAL_MEMORY_OFFSET
            .get()
            .expect("Physical memory offset request failed");

        let pml4_addr = Cr3::read().0.start_address().as_u64() + pmo;
        debug!("PML4 address: {:#x}", pml4_addr);
        let pml4 = unsafe { &mut *(pml4_addr as *mut PageTable) };

        let pagetable = unsafe { OffsetPageTable::new(pml4, VirtAddr::new(pmo)) };
        Mutex::new(pagetable)
    });
    info!("Offset page table initialized!");
    kernel::map_kernel()
}
