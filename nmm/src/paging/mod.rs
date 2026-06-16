//! Contains the core types and structures related to paging, such as page table entries, page tables, and the layout of the page table hierarchy. It also defines the virtual and physical address types used by the architecture.
pub mod builder;
pub mod index;
pub(crate) mod limine;
pub mod map;
pub mod primitives;
mod table;

use core::marker::Destruct;
use core::ops;

pub use table::{PageTable, PageTableEntry};

use cake::log::trace;
pub use index::PageTableIndex;

use crate::{
    MapFlags, MemError, NmmSealed,
    arch::{Mapper, PageEntryType},
    paging::map::{Flush, MemoryMapper},
    seal,
};

pub use primitives::{Frame, UnsizedFrame};
pub use primitives::{Page, UnsizedPage};

/// The virtual address type used by the current architecture.
pub type VirtAddr = primitives::VirtAddr;
/// The physical address type used by the current architecture.
pub type PhysAddr = primitives::PhysAddr;

/// The type used for page table entries in the current architecture.
pub type Table = [PageEntryType; crate::arch::ENTRY_COUNT];

/// A trait representing a memory primitive that can be used in paging, such as a page or a frame.
/// This trait is sealed to prevent external implementations, ensuring that only the intended types (like `Page` and `Frame`) can be used as memory primitives
/// in the paging system.
#[allow(private_bounds)] // intentionally seal this
pub const trait MemoryPrimitive<Ps: PrimitiveSize>: NmmSealed {
    /// The address space type associated with this memory primitive (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    type AddressSpace: Address;

    /// Returns the starting address of this memory primitive as the appropriate address space type (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    fn start_address(&self) -> Self::AddressSpace;
}

/// Helper trait to make AddressSpace's definition a little less gross
const trait AddrSpaceMath:
    Sized
    + [const] ops::Add<u64, Output = Self>
    + [const] ops::Sub<u64, Output = Self>
    + [const] ops::AddAssign<u64>
    + [const] ops::SubAssign<u64>
    + [const] ops::Add<Self>
    + [const] ops::Sub<Self>
    + [const] ops::AddAssign<Self>
    + [const] ops::SubAssign<Self>
{
}

impl<T> AddrSpaceMath for T where
    T: Sized
        + ops::Add<u64, Output = Self>
        + ops::Sub<u64, Output = Self>
        + ops::AddAssign<u64>
        + ops::SubAssign<u64>
        + ops::Add<Self>
        + ops::Sub<Self>
        + ops::AddAssign<Self>
        + ops::SubAssign<Self>
{
}

/// Address space primitives, e.g. `VirtAddr` and `PhysAddr`.
///
/// This is used for generic functions that can work with either virtual or physical addresses.
pub const trait Address:
    NmmSealed + Copy + core::fmt::Debug + Eq + PartialEq + Ord + PartialOrd + AddrSpaceMath
{
    /// Tries to create a new address from the given value.
    /// The value must be valid for the current architecture's address, otherwise this function will return `None`.
    fn try_new(val: u64) -> Option<Self>;

    /// Creates a new address from the given value.
    /// The value must be valid for the current architecture's address, otherwise this function will panic.
    fn new(val: u64) -> Self {
        Self::try_new(val).expect("AddressSpace::new: value is invalid for this address")
    }

    /// Creates a new address from the given value, truncating any bits beyond the architecture's bit width.
    fn new_truncate(val: u64) -> Self;

    /// Creates a new address from the given value without checking for validity.
    unsafe fn new_unchecked(val: u64) -> Self;

    /// Creates a new address from the given memory primitive.
    ///  The starting address of the primitive will be used as the value for the address.
    fn from_primitive<P: [const] MemoryPrimitive<S> + [const] Destruct, S: PrimitiveSize>(
        primitive: P,
    ) -> Option<Self>
    where
        P::AddressSpace: [const] Address,
    {
        let primitive_addr = primitive.start_address();
        Self::try_new(primitive_addr.as_u64())
    }

    /// Adds `rhs` to this address, returning a new address if the result is valid for the current architecture's address, or `None` if the result is invalid.
    fn checked_add(&self, rhs: u64) -> Option<Self> {
        match self.as_u64().checked_add(rhs) {
            Some(val) => Self::try_new(val),
            None => None,
        }
    }

    /// Returns the value of this address as a `u64`.
    fn as_u64(&self) -> u64;
}

/// Non-const additions to `Address` types.
pub trait AddressExt: Address {
    /// Creates a new address using `ptr` as the value for the address.
    fn from_ptr<T>(ptr: *const T) -> Option<Self> {
        Self::try_new(ptr as u64)
    }

    /// Creates a new address using `ptr` as the value for the address.
    fn from_mut_ptr<T>(ptr: *mut T) -> Option<Self> {
        Self::try_new(ptr as u64)
    }

    /// Returns the value of this address as a pointer of the given type.
    ///
    /// The returned pointer is not guaranteed to be valid for dereferencing, and the caller must ensure that any dereferencing of the returned pointer is safe.
    fn as_ptr<T>(&self) -> *const T {
        (self.as_u64() as usize) as *const T
    }

    /// Returns the value of this address as a pointer of the given type.
    ///
    /// The returned pointer is not guaranteed to be valid for dereferencing, and the caller must ensure that any dereferencing of the returned pointer is safe.
    fn as_mut_ptr<T>(&self) -> *mut T {
        (self.as_u64() as usize) as *mut T
    }
}

impl<T: Address> AddressExt for T {}

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
// I wish we didn't also have to specify the address space here..
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

/// Maps a memory primitive (such as a frame) to a page with the specified flags, using the provided frame allocator to allocate any necessary intermediate page tables.
#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
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
