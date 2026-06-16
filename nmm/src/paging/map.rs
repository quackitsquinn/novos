//! A trait for types that can map and unmap pages of a specific size. This is the main interface for mapping
//! and unmapping pages in the memory manager, and it abstracts over the architecture-specific details of how
//! page tables are manipulated to create mappings.

use crate::{
    MapFlags, MemError,
    paging::{Frame, Page, PrimitiveRangeManager, PrimitiveSize, Small, VirtAddr},
};

/// A trait for types that can map and unmap pages of a specific size. This is the main interface for mapping
/// and unmapping pages in the memory manager, and it abstracts over the architecture-specific details of how
/// page tables are manipulated to create mappings.
pub trait MemoryMapper<S: PrimitiveSize> {
    /// Maps the given page to the given frame with the specified flags, using the provided frame allocator
    /// for any necessary allocations of page tables.
    ///
    /// Returns an error if the mapping operation fails for any reason.
    fn map<A>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: MapFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: PrimitiveRangeManager<Frame<Small>, Small>;

    /// Unmaps the given page, returning the frame that was mapped to it before, or an error if the page was not mapped.
    unsafe fn unmap(&mut self, page: Page<S>) -> Result<(Frame<S>, Flush), MemError>;
}

/// A wrapper type for a virtual address that needs to be flushed from the TLB after a mapping operation.
/// This is used to ensure that the TLB is properly flushed after unmapping pages, which is necessary to prevent stale mappings from being used.
#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub struct Flush(FlushInner);

impl Flush {
    /// Creates a new `Flush` that indicates that all TLB entries should be flushed. This is used when unmapping large pages, where multiple TLB entries may be affected.
    pub fn flush_all() -> Self {
        Self(FlushInner::FlushAll)
    }

    /// Creates a new `Flush` for the given virtual address.
    ///
    /// # Safety
    ///
    pub unsafe fn flush_page<S: PrimitiveSize>(page: Page<S>) -> Self {
        Self(FlushInner::Flush(page.start_address()))
    }

    /// Flushes the TLB entry for the virtual address contained in this `Flush`, or flushes all TLB entries if this `Flush` indicates that all entries should be flushed.
    pub fn flush(self) {
        self.0.flush();
    }

    /// Ignores this `Flush`.
    pub fn ignore(self) {}
}

enum FlushInner {
    Flush(VirtAddr),
    FlushAll,
}

impl FlushInner {
    pub fn flush(self) {
        match self {
            Self::Flush(addr) => unsafe { crate::arch::do_flush(addr) },
            Self::FlushAll => unsafe { crate::arch::do_flush_all() },
        }
    }
}
