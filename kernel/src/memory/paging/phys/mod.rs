use core::error::Error;

use mapper::PageFrameAllocator;

use crate::{declare_module, util::OnceMutex};

pub mod mapper;
pub mod phys_mem;

pub static MEMORY_MAP_REQUEST: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

pub static FRAME_ALLOCATOR: OnceMutex<PageFrameAllocator> = OnceMutex::uninitialized();

declare_module!("physical memory mapping", init, &'static str);

fn init() -> Result<(), &'static str> {
    let mmap = MEMORY_MAP_REQUEST
        .get_response()
        .ok_or("Memory map not found")?;
    FRAME_ALLOCATOR.init(PageFrameAllocator::new(mmap));
    Ok(())
}
