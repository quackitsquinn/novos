//! A trait for types that can map and unmap pages of a specific size. This is the main interface for mapping
//! and unmapping pages in the memory manager, and it abstracts over the architecture-specific details of how
//! page tables are manipulated to create mappings.

use core::fmt;

use cake::log::trace;

use crate::{
    MapFlags, MemError,
    arch::Mapper,
    paging::{
        Address, FragmentManager, FragmentSize, Frame, FullManager, Large, Medium, MemoryFragment,
        Page, PhysAddr, Small, VirtAddr, asm,
        fragment::{GreedyFragmentMapper, JointFragmentMapper},
        operation::{Operation, OperationAllSizes},
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
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: FragmentManager<Frame<Small>, Small>;

    /// Unmaps the given page, returning the frame that was mapped to it before, or an error if the page was not mapped.
    unsafe fn unmap_primitive(&mut self, page: Page<S>) -> Result<Unmapped<S>, MemError>;
}

/// A provider for memory for a mapping operation.
pub trait MemoryProvider<S: FragmentSize> {
    /// Allocates a frame of the specified size for use as data in a mapping operation.
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError>
    where
        Mapper: SizedMemoryMapper<S>;
    /// Allocates a frame of the specified size for use as a page table in a mapping operation.
    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError>;
}

/// Memory provider to map a linear range of physical addresses to a linear range of virtual addresses.
#[derive(Debug)]
pub struct PhysLinear<'a, Ta: FragmentManager<Frame<Small>, Small>>(pub PhysAddr, pub &'a mut Ta);

impl<'a, Ta: FragmentManager<Frame<Small>, Small>> PhysLinear<'a, Ta> {
    /// Creates a new `Linear` memory provider with the given base physical address.
    pub fn new(base: PhysAddr, table_allocator: &'a mut Ta) -> Self {
        Self(base, table_allocator)
    }
}

impl<'a, Ta: FragmentManager<Frame<Small>, Small>, S: FragmentSize> MemoryProvider<S>
    for PhysLinear<'a, Ta>
{
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError> {
        let frame =
            Frame::from_start_address(self.0).ok_or(MemError::InvalidFrameAddress(self.0))?;
        self.0 += S::SIZE;
        Ok(frame)
    }

    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError> {
        self.1.allocate_fragment()
    }
}

/// A memory provider that uses a single allocator for both data and page tables.
#[derive(Debug)]
pub struct DataAllocator<'a, Ta>(pub &'a mut Ta)
where
    Ta: FullManager<FrameClass>;

impl<'a, Ta: FullManager<FrameClass>, S: FragmentSize> MemoryProvider<S> for DataAllocator<'a, Ta>
where
    Ta: FragmentManager<Frame<S>, S>,
{
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError> {
        self.0.allocate_fragment()
    }

    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError> {
        self.0.allocate_fragment()
    }
}

struct TableAllocator<'a, T: MemoryProvider<Small>>(&'a mut T);

impl<'a, T> TableAllocator<'a, T>
where
    T: MemoryProvider<Small>,
{
    fn new(provider: &'a mut T) -> Self {
        Self(provider)
    }
}

unsafe impl<'a, T> FragmentManager<Frame<Small>, Small> for TableAllocator<'a, T>
where
    T: MemoryProvider<Small>,
{
    fn allocate_fragment(&mut self) -> Result<Frame<Small>, MemError> {
        self.0.allocate_table()
    }

    fn deallocate_fragment(&mut self, primitive: Frame<Small>) {
        // Deallocation is not supported in this implementation.
        // In a real implementation, you would want to add support for deallocation.
        unimplemented!("Deallocation is not supported in this implementation.");
    }
}

/// A memory provider that uses separate allocators for data and page tables.
#[derive(Debug, Clone, Copy)]
pub struct DataWithTableAllocator<D, T>(pub D, pub T)
where
    D: FullManager<FrameClass>,
    T: FragmentManager<Frame<Small>, Small>;

impl<D: FullManager<FrameClass>, S: FragmentSize, T> MemoryProvider<S>
    for DataWithTableAllocator<D, T>
where
    D: FragmentManager<Frame<S>, S>,
    T: FragmentManager<Frame<Small>, Small>,
{
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError>
    where
        Mapper: SizedMemoryMapper<S>,
    {
        self.0.allocate_fragment()
    }

    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError> {
        self.1.allocate_fragment()
    }
}

pub(crate) trait FullProvider:
    MemoryProvider<Small> + MemoryProvider<Medium> + MemoryProvider<Large>
{
    fn table_allocator(&mut self) -> TableAllocator<'_, Self>
    where
        Self: Sized,
    {
        TableAllocator::new(self)
    }
}

impl<T> FullProvider for T where
    T: MemoryProvider<Small> + MemoryProvider<Medium> + MemoryProvider<Large>
{
}

/// A memory mapper that can map and unmap pages of any size.
pub trait MemoryMapper:
    SizedMemoryMapper<Small> + SizedMemoryMapper<Medium> + SizedMemoryMapper<Large>
{
    /// Maps a range of virtual addresses to physical frames, using the provided frame allocator for any necessary allocations of page tables.
    unsafe fn map_from<P: FullProvider>(
        &mut self,
        base: VirtAddr,
        len: u64,
        flags: MapFlags,
        provider: &mut P,
    ) -> Result<(), MemError> {
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
                    let frame = provider.allocate_data()?;
                    self.map_primitive(prim, frame, flags, &mut provider.table_allocator())?
                        .flush();
                }
                AnyFragment::Medium(prim) => {
                    let frame = provider.allocate_data()?;
                    self.map_primitive(prim, frame, flags, &mut provider.table_allocator())?
                        .flush();
                }
                AnyFragment::Large(prim) => {
                    let frame = provider.allocate_data()?;
                    self.map_primitive(prim, frame, flags, &mut provider.table_allocator())?
                        .flush();
                }
            }
        }

        Ok(())
    }

    /// Maps a range of virtual addresses to physical frames, using the provided frame allocator for any necessary allocations of page tables, and executes the given memory mapping operations for each mapping.
    unsafe fn map_from_with_operation<D>(
        &mut self,
        base: VirtAddr,
        len: u64,
        flags: MapFlags,
        data_allocator: &mut D,
        mut op: impl OperationAllSizes,
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
                    self.map_primitive(prim, frame, flags, data_allocator)?
                        .flush();
                    unsafe { op.execute(prim, frame)? };
                }
                AnyFragment::Medium(prim) => {
                    let frame = data_allocator.allocate_medium()?;
                    self.map_primitive(prim, frame, flags, data_allocator)?
                        .flush();
                    unsafe { op.execute(prim, frame)? };
                }
                AnyFragment::Large(prim) => {
                    let frame = data_allocator.allocate_large()?;
                    self.map_primitive(prim, frame, flags, data_allocator)?
                        .flush();
                    unsafe { op.execute(prim, frame)? };
                }
            }
        }

        Ok(())
    }
}

impl<T> MemoryMapper for T where
    T: SizedMemoryMapper<Small> + SizedMemoryMapper<Medium> + SizedMemoryMapper<Large>
{
}

/// A structure representing an unmapped page.
#[derive(Debug)]
pub struct Unmapped<S: FragmentSize> {
    /// The physical frame that was previously mapped to the page.
    pub mut(crate) frame: Frame<S>,
    flush: Option<Flush>,
    /// The mapping flags that were used for the mapping before it was unmapped.
    pub mut(crate) flags: MapFlags,
}

impl<S: FragmentSize> Unmapped<S> {
    /// Creates a new `Unmapped` structure with the given frame, flush operation, and mapping flags.
    pub fn new(frame: Frame<S>, flush: Option<Flush>, flags: MapFlags) -> Self {
        Self {
            frame,
            flush,
            flags,
        }
    }

    /// Flushes the TLB entry for the unmapped page, if it has not already been flushed.
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
