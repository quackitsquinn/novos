
use limine::{memory_map::EntryType, paging::Mode, response::MemoryMapResponse};
use log::{error, info};
use spin::Once;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::{MapToError, MapperFlush},
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::util::OnceMutex;

pub mod virt;

#[used]
static PAGE_TABLE_REQUEST: limine::request::PagingModeRequest =
    limine::request::PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
static MEMORY_OFFSET_REQUEST: limine::request::HhdmRequest = limine::request::HhdmRequest::new();

#[used]
static MEMORY_MAP_REQUEST: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

pub(super) static FRAME_ALLOCATOR: OnceMutex<PageFrameAllocator> = OnceMutex::new();

pub static MEMORY_OFFSET: Once<u64> = Once::new();
pub(super) static OFFSET_PAGE_TABLE: OnceMutex<OffsetPageTable> = OnceMutex::new();

pub(super) struct PageFrameAllocator {
    mmap: &'static MemoryMapResponse,
    off: usize,
    // TODO: when we have a heap, use a Box<Iterator<Item = PhysFrame>> to cache the usable frames
}

impl PageFrameAllocator {
    pub fn new(mmap: &'static MemoryMapResponse) -> Self {
        Self { mmap, off: 0 }
    }

    pub fn usable_frames(&mut self) -> impl Iterator<Item = PhysFrame> {
        self.mmap
            .entries()
            .iter()
            .filter(|e| e.entry_type == EntryType::USABLE)
            .map(|e| (e.base..e.base + e.length))
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    pub fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size4KiB>, &'static str> {
        let mut pgtbl = OFFSET_PAGE_TABLE.get();
        let frame = self.allocate_frame().ok_or("Out of frames")?;
        Ok(
            unsafe { pgtbl.map_to(page, frame, flags, &mut *self) }.map_err(|e| {
                error!("Error mapping page: {:?}", e);
                "Error mapping page"
            })?,
        )
    }

    pub unsafe fn identity_map(
        &mut self,
        frame: PhysFrame,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size4KiB>, MapToError<Size4KiB>> {
        let mut pgtbl = OFFSET_PAGE_TABLE.get();
        unsafe { pgtbl.identity_map(frame, flags, &mut *self) }
    }
}

unsafe impl FrameAllocator<Size4KiB> for PageFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.off += 1;
        self.usable_frames().nth(self.off - 1)
    }
}

pub(super) fn init() {
    // init order will probably be serial -> paging -> everything else
    // TODO: refactor this module into a phys module and update this init function
    info!("Initializing paging");
    let off = MEMORY_OFFSET_REQUEST.get_response().unwrap().offset();
    MEMORY_OFFSET.call_once(|| off);
    info!("Set memory offset to 0x{:x}", off);
    let cr3 = Cr3::read();
    info!("Current CR3: {:?}", cr3);
    let pgtbl = unsafe { &mut *((cr3.0.start_address().as_u64() + off) as *mut PageTable) };
    OFFSET_PAGE_TABLE.init(unsafe { OffsetPageTable::new(pgtbl, VirtAddr::new(off)) });
    FRAME_ALLOCATOR.init(PageFrameAllocator::new(
        MEMORY_MAP_REQUEST.get_response().unwrap(),
    ));
}
