use core::mem;

use bitflags::iter::Iter;
use limine::{memory_map::EntryType, paging::Mode, response::MemoryMapResponse};
use spin::{Mutex, Once};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::MapperFlush, page::PageRangeInclusive, FrameAllocator, Mapper, OffsetPageTable,
        Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::{sprint, sprintln, util::OnceMutex};

pub mod allocator;

// Evaluates to 0x4156_4F4E_0000
pub const HEAP_MEM_OFFSET: u64 = (u32::from_ne_bytes(*b"NOVA") as u64) << 16;
pub const HEAP_SIZE: u64 = 1024 * 512; // 512 KiB

#[used]
static PAGE_TABLE_REQUEST: limine::request::PagingModeRequest =
    limine::request::PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
static MEMORY_OFFSET_REQUEST: limine::request::HhdmRequest = limine::request::HhdmRequest::new();

#[used]
static MEMORY_MAP_REQUEST: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

static FRAME_ALLOCATOR: OnceMutex<PageFrameAllocator> = OnceMutex::new();

static MEMORY_OFFSET: Once<u64> = Once::new();
static OFFSET_PAGE_TABLE: OnceMutex<OffsetPageTable> = OnceMutex::new();

pub fn init() {
    // init order will probably be serial -> paging -> everything else
    sprintln!("Initializing paging");
    let off = MEMORY_OFFSET_REQUEST.get_response().unwrap().offset();
    MEMORY_OFFSET.call_once(|| off);
    sprintln!("Set memory offset to 0x{:x}", off);
    let cr3 = Cr3::read();
    sprintln!("Current CR3: {:?}", cr3);
    let pgtbl = unsafe { &mut *((cr3.0.start_address().as_u64() + off) as *mut PageTable) };
    OFFSET_PAGE_TABLE.init(unsafe { OffsetPageTable::new(pgtbl, VirtAddr::new(off)) });
    FRAME_ALLOCATOR.init(PageFrameAllocator::new(
        MEMORY_MAP_REQUEST.get_response().unwrap(),
    ));
    sprintln!(
        "Initialized paging.. Mapping 0x{:x} byte heap at 0x{:x}",
        HEAP_SIZE,
        HEAP_MEM_OFFSET
    );
    init_heap();
}

struct PageFrameAllocator {
    mmap: &'static MemoryMapResponse,
    off: usize,
    // TODO: when we have a heap, use a Box<Iterator<Item = PhysFrame>> to cache the usable frames
}

impl PageFrameAllocator {
    pub fn new(mmap: &'static MemoryMapResponse) -> Self {
        Self { mmap, off: 0 }
    }

    fn usable_frames(&mut self) -> impl Iterator<Item = PhysFrame> {
        self.mmap
            .entries()
            .iter()
            .filter(|e| e.entry_type == EntryType::USABLE)
            .map(|e| (e.base..e.base + e.length))
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size4KiB>, &'static str> {
        let mut pgtbl = OFFSET_PAGE_TABLE.get();
        let frame = self.allocate_frame().ok_or("Out of frames")?;
        Ok(
            unsafe { pgtbl.map_to(page, frame, flags, &mut *self) }.map_err(|e| {
                sprintln!("Error mapping page: {:?}", e);
                "Error mapping page"
            })?,
        )
    }
}

unsafe impl FrameAllocator<Size4KiB> for PageFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.off += 1;
        self.usable_frames().nth(self.off - 1)
    }
}

fn init_heap() {
    let hstart = VirtAddr::new(HEAP_MEM_OFFSET);
    let hend = hstart + HEAP_SIZE - 1u64;
    let hstart_page = Page::containing_address(hstart);
    let hend_page = Page::containing_address(hend);
    let hrange: PageRangeInclusive<Size4KiB> = Page::range_inclusive(hstart_page, hend_page);

    let mut pfa = FRAME_ALLOCATOR.get();
    for page in hrange {
        pfa.map_page(page, PageTableFlags::PRESENT | PageTableFlags::WRITABLE)
            .unwrap()
            .flush();
    }
}
