use cake::{Mutex, Once, debug, info};
use kvmm::{
    KernelPage,
    phys::{PhysAddrRange, frame_mapper::FrameMapper},
    virt::alloc::SimplePageAllocator,
};
use limine::memory_map::{Entry, EntryType};

use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, frame},
};

mod alloc;
mod kernel;
mod map;

pub(crate) use kernel::Kernel;

use crate::{mem::alloc::TrampolineAllocator, requests::MEMORY_MAP};

const ALLOC_BASE: VirtAddr = VirtAddr::new(0x100_000_100);
const ALLOC_PAGES: usize = 256; // 1 MiB

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
#[global_allocator]
pub(crate) static ALLOCATOR: TrampolineAllocator = TrampolineAllocator::new();

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
    init_alloc();
    map::map_kernel()
}

fn init_alloc() {
    info!("Initializing global allocator...");

    let base_page = KernelPage::containing_address(ALLOC_BASE);
    let end_page = KernelPage::containing_address(ALLOC_BASE + 4096 * ALLOC_PAGES as u64);
    let page_range = base_page..end_page;

    let mut mapper_guard = PAGETABLE.get().expect("PAGETABLE not initialized").lock();
    let mut frame_mapper_guard = MAPPER.get().expect("MAPPER not initialized").lock();
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;

    for page in page_range {
        let frame = frame_mapper_guard
            .next_frame()
            .expect("Out of memory: could not allocate frame for global allocator");
        unsafe {
            mapper_guard
                .map_to(page, frame, flags, &mut *frame_mapper_guard)
                .expect("map_to failed")
                .flush();
        }
    }

    let allocator = unsafe { SimplePageAllocator::new(base_page, ALLOC_PAGES) };
    ALLOCATOR.init(allocator);
    info!("Global allocator initialized!");
}
