//! A trait for types that can map and unmap pages of a specific size. This is the main interface for mapping
//! and unmapping pages in the memory manager, and it abstracts over the architecture-specific details of how
//! page tables are manipulated to create mappings.

use core::fmt;

use cake::log::trace;

use crate::{
    MapFlags, MemError,
    paging::{
        Address, EntryMappingFlags, FragmentManager, FragmentSize, Frame, FullManager, Large,
        Medium, Page, PhysAddr, Small, VirtAddr,
        fragment::{GreedyFragmentMapper, JointFragmentMapper},
        primitives::{AnyFragment, FrameClass, PageClass},
    },
};

/// A trait for types that can map and unmap pages of a specific size. This is the main interface for mapping
/// and unmapping pages in the memory manager, and it abstracts over the architecture-specific details of how
/// page tables are manipulated to create mappings.
pub trait SizedMemoryMapper<S: FragmentSize> {
    /// Maps the given page to the given frame with the specified flags, using the provided frame allocator
    /// for any necessary allocations of page tables.
    ///
    /// Returns an error if the mapping operation fails for any reason.
    fn map_primitive<A>(
        &mut self,
        dst: Page<S>,
        src: Frame<S>,
        flags: MapFlags,
        mapping_flags: EntryMappingFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: FragmentManager<Frame<Small>, Small>;

    /// Unmaps the given page, returning the frame that was mapped to it before, or an error if the page was not mapped.
    unsafe fn unmap_primitive(&mut self, page: Page<S>) -> Result<Unmapped<S>, MemError>;
}

/// A memory mapper that can map and unmap pages of any size.
pub trait MemoryMapper:
    SizedMemoryMapper<Small> + SizedMemoryMapper<Medium> + SizedMemoryMapper<Large>
{
    /// Maps a range of virtual addresses to physical frames, using the provided frame allocator for any necessary allocations of page tables.
    unsafe fn map_from<D>(
        &mut self,
        base: VirtAddr,
        len: u64,
        flags: MapFlags,
        mapping_flags: EntryMappingFlags,
        data_allocator: &mut D,
    ) -> Result<(), MemError>
    where
        D: FullManager<FrameClass>,
    {
        trace!(
            "Mapping from base address {:x?} with length {:?} and flags {:?}",
            base.as_u64(),
            len,
            flags
        );

        let mapper = GreedyFragmentMapper::<PageClass>::new(base, len);
        for frag in mapper {
            match frag {
                AnyFragment::Small(prim) => {
                    let frame = data_allocator.allocate_small()?;
                    self.map_primitive(prim, frame, flags, mapping_flags, data_allocator)?
                        .flush();
                }
                AnyFragment::Medium(prim) => {
                    let frame = data_allocator.allocate_medium()?;
                    self.map_primitive(prim, frame, flags, mapping_flags, data_allocator)?
                        .flush();
                }
                AnyFragment::Large(prim) => {
                    let frame = data_allocator.allocate_large()?;
                    self.map_primitive(prim, frame, flags, mapping_flags, data_allocator)?
                        .flush();
                }
            }
        }

        Ok(())
    }

    /// Maps a range of virtual addresses to physical frames, using the provided frame allocator for any necessary allocations of page tables.
    unsafe fn map<F>(
        &mut self,
        virt_base: VirtAddr,
        phys_base: PhysAddr,
        byte_size: usize,
        flags: MapFlags,
        mapping_flags: EntryMappingFlags,
        frame_alloc: &mut F,
    ) -> Result<(), MemError>
    where
        F: FragmentManager<Frame<Small>, Small>,
    {
        let mapper = JointFragmentMapper::new(virt_base, phys_base, byte_size as u64);

        for pair in mapper {
            match pair {
                (AnyFragment::Small(page_prim), AnyFragment::Small(phys_prim)) => {
                    self.map_primitive(page_prim, phys_prim, flags, mapping_flags, frame_alloc)?
                        .flush();
                }
                (AnyFragment::Medium(page_prim), AnyFragment::Medium(phys_prim)) => {
                    self.map_primitive(page_prim, phys_prim, flags, mapping_flags, frame_alloc)?
                        .flush();
                }
                (AnyFragment::Large(page_prim), AnyFragment::Large(phys_prim)) => {
                    self.map_primitive(page_prim, phys_prim, flags, mapping_flags, frame_alloc)?
                        .flush();
                }
                _ => unreachable!("non-matched fragments produced by mapper"),
            }
        }

        Ok(())
    }
}

impl<T> MemoryMapper for T where
    T: SizedMemoryMapper<Small> + SizedMemoryMapper<Medium> + SizedMemoryMapper<Large>
{
}

pub struct Unmapped<S: FragmentSize> {
    pub frame: Frame<S>,
    flush: Option<Flush>,
    pub mapping_flags: EntryMappingFlags,
}

impl<S: FragmentSize> Unmapped<S> {
    pub fn new(frame: Frame<S>, flush: Option<Flush>, mapping_flags: EntryMappingFlags) -> Self {
        Self {
            frame,
            flush,
            mapping_flags,
        }
    }

    pub fn flush(&mut self) {
        self.flush.take().map(|f| f.flush());
    }
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
    pub unsafe fn flush_page<S: FragmentSize>(page: Page<S>) -> Self {
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

impl fmt::Debug for Flush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            FlushInner::Flush(addr) => write!(f, "Flush({:?})", addr),
            FlushInner::FlushAll => write!(f, "FlushAll"),
        }
    }
}
