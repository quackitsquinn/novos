use crate::{
    MapFlags, MemError,
    paging::{Frame, Page, PrimitiveRangeManager, PrimitiveSize},
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
    ) -> Result<(), MemError>
    where
        A: PrimitiveRangeManager<Frame<S>, S>;

    /// Unmaps the given page, returning the frame that was mapped to it before, or an error if the page was not mapped.
    unsafe fn unmap(&mut self, page: Page<S>) -> Result<Frame<S>, MemError>;
}
