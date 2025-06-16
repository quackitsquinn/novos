use mapper::PageFrameAllocator;

use crate::{declare_module, requests::MEMORY_MAP, util::OnceMutex};

pub mod mapper;
pub mod phys_mem;

pub static FRAME_ALLOCATOR: OnceMutex<PageFrameAllocator> = OnceMutex::uninitialized();

declare_module!("physical memory mapping", init, &'static str);

fn init() -> Result<(), &'static str> {
    FRAME_ALLOCATOR.init(PageFrameAllocator::new(
        MEMORY_MAP.get().expect("memory map uninit"),
    ));
    Ok(())
}
