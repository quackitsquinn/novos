use core::alloc::GlobalAlloc;

use cake::{Mutex, OnceMutex, error, info};
use kvmm::virt::alloc::{SimplePageAllocator, layout_to_pages, usize_to_pages};

pub struct TrampolineAllocator {
    inner: OnceMutex<SimplePageAllocator>,
}

impl TrampolineAllocator {
    pub const fn new() -> Self {
        TrampolineAllocator {
            inner: OnceMutex::uninitialized(),
        }
    }

    pub fn init(&self, allocator: SimplePageAllocator) {
        self.inner.init(allocator);
        info!("Trampoline allocator initialized");
    }
}

unsafe impl GlobalAlloc for TrampolineAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.align() > 4096 {
            error!("alloc failed: unsupported alignment {}", layout.align());
            return core::ptr::null_mut(); //// !! Unsupported alignment !!
        }
        let mut guard = self.inner.get();
        unsafe { guard.alloc(layout_to_pages(layout)) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if layout.align() > 4096 {
            error!("dealloc failed: unsupported alignment {}", layout.align());
            return;
        }
        let mut guard = self.inner.get();
        unsafe { guard.dealloc(ptr, layout_to_pages(layout)) }
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let mut guard = self.inner.get();
        if layout.align() > 4096 {
            error!("realloc failed: unsupported alignment {}", layout.align());
            return core::ptr::null_mut();
        }

        let pages = layout_to_pages(layout);
        let new_pages = usize_to_pages(new_size);

        // If the number of pages is the same, just return the same pointer.
        if pages == new_pages {
            return ptr;
        }

        unsafe { guard.realloc(ptr, pages, new_pages) }
    }
}
