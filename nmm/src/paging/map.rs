use crate::{
    MapFlags, MemError,
    paging::{Frame, Page, PrimitiveRangeManager, PrimitiveSize, Small, VirtAddr},
};

/// A trait for types that can map and unmap pages of a specific size. T
/// his is the main interface for mapping and unmapping pages in the memory manager, and it abstracts over
/// the architecture-specific details of how page tables are manipulated to create mappings.
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
pub struct Flush(VirtAddr);

impl Flush {
    /// Creates a new `Flush` for the given virtual address. The caller must ensure that the provided virtual address is the base address of the page that was unmapped, and that it is properly aligned to the page size.
    pub unsafe fn new(addr: VirtAddr) -> Self {
        Self(addr)
    }

    pub fn flush(self) {
        // SAFETY: The caller must ensure that the provided virtual address is the base address of the page that was unmapped, and that it is properly aligned to the page size. If this is not the case, flushing the TLB with an invalid address could cause undefined behavior.
        unsafe { crate::arch::do_flush(self.0) }
    }

    pub fn ignore(self) {}
}
