//! Contains the core types and structures related to paging, such as page table entries, page tables, and the layout of the page table hierarchy. It also defines the virtual and physical address types used by the architecture.
pub mod builder;
mod fragment;
pub mod index;
pub(crate) mod limine;
pub mod map;
pub mod primitives;
mod table;


pub use table::{PageTable, PageTableEntry};

use cake::log::trace;
pub use index::PageTableIndex;

use crate::{
    MapFlags, MemError,
    arch::{Mapper, PageEntryType},
    paging::{
        fragment::GreedyFragmentMapper,
        map::{Flush, MemoryMapper},
        primitives::{AnyPrimitive, PageClass},
    },
};

pub use primitives::{Address, AddressExt};
pub use primitives::{Frame, UnsizedFrame};
pub use primitives::{Large, Medium, MemoryFragment, PrimitiveSize, Small};
pub use primitives::{Page, UnsizedPage};
pub use primitives::{PhysAddr, VirtAddr};

/// The type used for page table entries in the current architecture.
pub type Table = [PageEntryType; crate::arch::ENTRY_COUNT];

/// A trait for managing ranges of memory primitives, such as pages. This is used for allocating and deallocating pages of different sizes, and can be implemented by both the physical and virtual memory managers to manage their respective address spaces.
// I wish we didn't also have to specify the address space here..
#[allow(private_bounds)] // intentionally seal this
pub trait PrimitiveRangeManager<T: MemoryFragment<S>, S: PrimitiveSize> {
    /// Allocates a range of memory of the specified size and alignment, returning the starting address of the allocated range.
    fn allocate_range(&mut self) -> Option<T>;
    /// Deallocates a previously allocated range of memory, given the starting address and size of the range.
    fn deallocate_range(&mut self, primitive: T);
}

/// A marker trait for types that implement `PrimitiveRangeManager<Page<S>>` for all page sizes.
pub trait AllPages:
    PrimitiveRangeManager<Page<Small>, Small>
    + PrimitiveRangeManager<Page<Medium>, Medium>
    + PrimitiveRangeManager<Page<Large>, Large>
{
    /// Allocates a small page (4KB on x86_64) and returns it, or `None` if no small pages are available.
    fn allocate_small(&mut self) -> Option<Page<Small>> {
        self.allocate_range()
    }

    /// Allocates a medium page (2MB on x86_64) and returns it, or `None` if no medium pages are available.
    fn allocate_medium(&mut self) -> Option<Page<Medium>> {
        self.allocate_range()
    }

    /// Allocates a large page (1GB on x86_64) and returns it, or `None` if no large pages are available.
    fn allocate_large(&mut self) -> Option<Page<Large>> {
        self.allocate_range()
    }
}

impl<T: PrimitiveSize> AllPages for T where
    T: PrimitiveRangeManager<Page<Small>, Small>
        + PrimitiveRangeManager<Page<Medium>, Medium>
        + PrimitiveRangeManager<Page<Large>, Large>
{
}

/// A marker trait for types that implement `PrimitiveRangeManager<Frame<S>>` for all page sizes.
pub trait AllFrames:
    PrimitiveRangeManager<Frame<Small>, Small>
    + PrimitiveRangeManager<Frame<Medium>, Medium>
    + PrimitiveRangeManager<Frame<Large>, Large>
{
    /// Allocates a small frame (4KB on x86_64) and returns it, or `None` if no small frames are available.
    fn allocate_small(&mut self) -> Option<Frame<Small>> {
        self.allocate_range()
    }

    /// Allocates a medium frame (2MB on x86_64) and returns it, or `None` if no medium frames are available.
    fn allocate_medium(&mut self) -> Option<Frame<Medium>> {
        self.allocate_range()
    }

    /// Allocates a large frame (1GB on x86_64) and returns it, or `None` if no large frames are available.
    fn allocate_large(&mut self) -> Option<Frame<Large>> {
        self.allocate_range()
    }
}

impl<T> AllFrames for T where
    T: PrimitiveRangeManager<Frame<Small>, Small>
        + PrimitiveRangeManager<Frame<Medium>, Medium>
        + PrimitiveRangeManager<Frame<Large>, Large>
{
}

/// Maps a memory primitive (such as a frame) to a page with the specified flags, using the provided frame allocator to allocate any necessary intermediate page tables.
//#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub(crate) fn map_primitive<S, A>(
    src: Frame<S>,
    dst: Page<S>,
    flags: MapFlags,
    frame_allocator: &mut A,
) -> Result<Flush, MemError>
where
    S: PrimitiveSize,
    A: PrimitiveRangeManager<Frame<Small>, Small>,
    Mapper: MemoryMapper<S>,
{
    trace!(
        "Mapping frame {:?} to page {:?} with flags {:?}",
        src, dst, flags
    );

    crate::arch::map_primitive(src, dst, flags, frame_allocator)
}

/// Unmaps a page, returning the frame that was mapped to it before, or an error if the page was not mapped.
///
/// # Safety
///
/// The caller must ensure that there are no currently living references to the memory that was mapped to the page being unmapped,
/// as accessing that memory afterwards is undefined behavior.
#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub(crate) unsafe fn unmap_primitive<S>(dst: Page<S>) -> Result<(Frame<S>, Flush), MemError>
where
    S: PrimitiveSize,
    Mapper: MemoryMapper<S>,
{
    trace!("Unmapping page {:?}", dst);

    unsafe { crate::arch::unmap_primitive(dst) }
}

pub(crate) unsafe fn map_from<D, F>(
    base: VirtAddr,
    len: u64,
    flags: MapFlags,
    data_allocator: &mut D,
) -> Result<(), MemError>
where
    D: AllFrames,
{
    trace!(
        "Mapping from base address {:?} with length {:?} and flags {:?}",
        base, len, flags
    );

    let mapper = GreedyFragmentMapper::<PageClass>::new(base, len);
    for frag in mapper {
        match frag {
            AnyPrimitive::Small(prim) => {
                let frame = data_allocator
                    .allocate_small()
                    .ok_or(MemError::OutOfMemory)?;
                map_primitive(frame, prim, flags, data_allocator)?.flush();
            }
            AnyPrimitive::Medium(prim) => {
                let frame = data_allocator
                    .allocate_medium()
                    .ok_or(MemError::OutOfMemory)?;
                map_primitive(frame, prim, flags, data_allocator)?.flush();
            }
            AnyPrimitive::Large(prim) => {
                let frame = data_allocator
                    .allocate_large()
                    .ok_or(MemError::OutOfMemory)?;
                map_primitive(frame, prim, flags, data_allocator)?.flush();
            }
        }
    }

    Ok(())
}

pub(crate) unsafe fn map_from_with_allocator<D, F>(
    base: VirtAddr,
    len: u64,
    flags: MapFlags,
    data_allocator: &mut D,
    frame_allocator: &mut F,
) -> Result<(), MemError>
where
    D: AllFrames,
    F: PrimitiveRangeManager<Frame<Small>, Small>,
{
    trace!(
        "Mapping from base address {:?} with length {:?} and flags {:?}",
        base, len, flags
    );

    let mapper = GreedyFragmentMapper::<PageClass>::new(base, len);
    for frag in mapper {
        match frag {
            AnyPrimitive::Small(prim) => {
                let frame = data_allocator
                    .allocate_small()
                    .ok_or(MemError::OutOfMemory)?;
                map_primitive(frame, prim, flags, frame_allocator)?.flush();
            }
            AnyPrimitive::Medium(prim) => {
                let frame = data_allocator
                    .allocate_medium()
                    .ok_or(MemError::OutOfMemory)?;
                map_primitive(frame, prim, flags, frame_allocator)?.flush();
            }
            AnyPrimitive::Large(prim) => {
                let frame = data_allocator
                    .allocate_large()
                    .ok_or(MemError::OutOfMemory)?;
                map_primitive(frame, prim, flags, frame_allocator)?.flush();
            }
        }
    }

    Ok(())
}
