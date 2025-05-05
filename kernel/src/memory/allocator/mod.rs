use core::{mem, ptr};

use kalloc::{block_alloc::allocator::BlockAllocator, GlobalAllocatorWrapper};
use log::{debug, error};
use x86_64::{structures::paging::Page, VirtAddr};

use super::paging;

#[global_allocator]
pub static ALLOCATOR: GlobalAllocatorWrapper<BlockAllocator> = GlobalAllocatorWrapper::new();

pub unsafe fn init(heap_start: *mut u8, heap_end: *mut u8) {
    //Command::InitIncrementalSend("alloc", "heap_snap{{ID}}.bin").send();
    kalloc::set_frame_output_fn(frame_output);
    let mut unmap = false;
    let ceiled_heap_end = heap_end as usize + (heap_end as usize % 4096);
    for addr in (heap_start as usize..ceiled_heap_end as usize).step_by(4096) {
        let mapped = paging::phys::FRAME_ALLOCATOR
            .get()
            .is_page_mapped(Page::containing_address(VirtAddr::new(addr as u64)))
            .is_some();
        if !mapped {
            error!("Page {:#x} is not mapped", addr);
            unmap = true;
        }
    }
    if unmap {
        panic!("Some pages in the heap are not mapped");
    }

    // Write 0xAA to the heap as uninit marker
    // for addr in (heap_start as usize..ceiled_heap_end as usize).step_by(4096) {
    //     unsafe {
    //         debug!("Writing 0xAA to heap at {:#x}", addr);
    //         ptr::write_bytes(addr as *mut u8, 0xAA, 4096);
    //     }
    // }

    ALLOCATOR.init(|| unsafe { BlockAllocator::init(heap_start.cast(), heap_end.cast(), false) });
}

pub fn frame_output(data: &[u8]) {
    //Command::SendIncrementalData("alloc", data).send();
}
