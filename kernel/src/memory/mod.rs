use core::alloc::GlobalAlloc;

pub struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Do nothing
    }
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;
