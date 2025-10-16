//! Physical memory mapping + page frame allocator
use cake::OnceMutex;
use mapper::PhysFrameAllocator;

use crate::{declare_module, requests::MEMORY_MAP};

pub(crate) mod mapper;
pub mod phys_mem;

pub(crate) static FRAME_ALLOCATOR: OnceMutex<PhysFrameAllocator> = OnceMutex::uninitialized();

declare_module!("physical memory mapping", init, &'static str);

fn init() -> Result<(), &'static str> {
    FRAME_ALLOCATOR.init(PhysFrameAllocator::new(MEMORY_MAP.get()));
    Ok(())
}
