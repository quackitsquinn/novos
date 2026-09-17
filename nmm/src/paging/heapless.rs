//! Early Heap-like structures for use before the heap is initialized.
//!
//! Be warned: every allocation has a footprint of at least `crate::arch::L1_PAGE_SIZE` bytes.

use core::alloc::Layout;

use alloc::alloc::Allocator;
use cake::log::error;

use crate::{
    MapFlags, MapSource, align,
    paging::{AddressExt, primitives::VirtRange},
};

/// A page backed allocator that does not require a heap to be initialized.
/// This is useful for early bootstrapping of the memory manager, where we need to allocate memory before the heap is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaplessAllocator(MapFlags);

impl HeaplessAllocator {
    /// Creates a new `HeaplessAllocator` with the given `MapFlags`.
    pub const fn new(flags: MapFlags) -> Self {
        Self(flags)
    }

    fn reallocate_fallback(
        &self,
        ptr: core::ptr::NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let new_ptr = self.allocate(new_layout)?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                ptr.as_ptr(),
                new_ptr.cast::<u8>().as_ptr(),
                old_layout.size(),
            );
            self.deallocate(ptr, old_layout);
        }
        Ok(new_ptr)
    }
}

unsafe impl Allocator for HeaplessAllocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let layout = create_page_layout(layout);
        let vbase = crate::reserve_virtual(layout).map_err(|_| alloc::alloc::AllocError)?;

        if let Err(e) = crate::map(vbase, MapSource::Anon, self.0, &mut ()) {
            // SAFETY: We just reserved this virtual memory, so it is safe to free it.
            error!(
                "Failed to map virtual memory for heapless allocator: {:?}",
                e
            );
            if let Err(e) = unsafe { crate::free_virtual(vbase, layout) } {
                error!(
                    "Failed to free virtual memory for heapless allocator: {:?}",
                    e
                );
            }
            return Err(alloc::alloc::AllocError);
        }

        let ptr = vbase.start().as_mut_ptr::<u8>();
        let slice = core::ptr::slice_from_raw_parts_mut(ptr, layout.size());
        Ok(core::ptr::NonNull::new(slice).unwrap())
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let layout = create_page_layout(layout);
        let vbase = crate::VirtAddr::from_ptr(ptr.as_ptr()).unwrap();
        // SAFETY: Upheld by the caller
        if let Err(e) = unsafe { crate::unmap(VirtRange::new_len(vbase, layout.size() as u64)) } {
            error!(
                "Failed to unmap virtual memory for heapless allocator: {:?}",
                e
            );
        }
    }

    unsafe fn grow(
        &self,
        ptr: core::ptr::NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let old_pl = create_page_layout(old_layout);
        let new_pl = create_page_layout(new_layout);

        if new_pl.size() <= old_pl.size() {
            return Ok(core::ptr::NonNull::new(core::ptr::slice_from_raw_parts_mut(
                ptr.as_ptr(),
                new_pl.size(),
            ))
            .unwrap());
        }

        self.reallocate_fallback(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(
        &self,
        ptr: core::ptr::NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let old_pl = create_page_layout(old_layout);
        let new_pl = create_page_layout(new_layout);

        if new_pl.size() >= old_pl.size() {
            return Ok(core::ptr::NonNull::new(core::ptr::slice_from_raw_parts_mut(
                ptr.as_ptr(),
                new_pl.size(),
            ))
            .unwrap());
        }

        self.reallocate_fallback(ptr, old_layout, new_layout)
    }
}

fn create_page_layout(layout: Layout) -> Layout {
    let size_pages = align!(up, layout.size(), crate::arch::L1_PAGE_SIZE as usize);
    let align_pages = usize::max(layout.align(), crate::arch::L1_PAGE_SIZE as usize);

    Layout::from_size_align(size_pages, align_pages).unwrap()
}
