//! Contains the core types and structures related to paging, such as page table entries, page tables, and the layout of the page table hierarchy. It also defines the virtual and physical address types used by the architecture.
mod frame;
pub mod index;
pub(crate) mod limine;
mod page;

use core::any::TypeId;

use cake::log::debug;
pub use index::PageTableIndex;

use crate::{NmmSealed, arch::PageEntryType, seal};

pub use frame::Frame;
pub use page::Page;

/// The virtual address type used by the current architecture.
pub type VirtAddr = crate::arch::VirtAddr;
/// The physical address type used by the current architecture.
pub type PhysAddr = crate::arch::PhysAddr;

/// The type used for page table entries in the current architecture.
pub type Table = [PageEntryType; crate::arch::ENTRY_COUNT];

/// A trait representing a memory primitive that can be used in paging, such as a page or a frame.
/// This trait is sealed to prevent external implementations, ensuring that only the intended types (like `Page` and `Frame`) can be used as memory primitives
/// in the paging system.
#[allow(private_bounds)] // intentionally seal this
pub trait MemoryPrimitive<S: PrimitiveSize>: NmmSealed {}

/// A trait representing a page size for the current architecture.
#[allow(private_bounds)]
pub trait PrimitiveSize: NmmSealed {
    /// The size of a page for this page size type, in bytes.
    const SIZE: u64;
}

/// Marker type for small pages, typically 4KB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Small;
impl PrimitiveSize for Small {
    const SIZE: u64 = crate::arch::L1_PAGE_SIZE;
}
/// Marker type for medium pages, typically 2MB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Medium;
impl PrimitiveSize for Medium {
    const SIZE: u64 = crate::arch::L2_PAGE_SIZE;
}
/// Marker type for large pages, typically 1GB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Large;
impl PrimitiveSize for Large {
    const SIZE: u64 = crate::arch::L3_PAGE_SIZE;
}

seal!(Small, Medium, Large);

/// A trait for managing ranges of memory primitives, such as pages. This is used for allocating and deallocating pages of different sizes, and can be implemented by both the physical and virtual memory managers to manage their respective address spaces.
#[allow(private_bounds)] // intentionally seal this
pub trait PrimitiveRangeManager<T: MemoryPrimitive<S>, S: PrimitiveSize> {
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
