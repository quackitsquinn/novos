use core::mem;

use crate::{
    MapFlags, MemError,
    arch::{
        PhysAddr,
        x86_64::{ArchError, PageTableFlags},
    },
    paging::{
        Frame, Large, Medium, Page, PageTable, PageTableIndex, PrimitiveRangeManager,
        PrimitiveSize, Small,
    },
};

mod arch_lib {
    pub use x86_64::structures::paging::{
        FrameAllocator, Page, PageSize, PageTable, PageTableFlags, PageTableIndex, PhysFrame,
        Size1GiB, Size2MiB, Size4KiB, mapper::MapToError, mapper::UnmapError,
    };
}

pub(super) struct XFrameAllocator<'a, S: PrimitiveSize, T>
where
    T: PrimitiveRangeManager<Frame<S>, S>,
{
    frame_range_manager: &'a mut T,
    _size_marker: core::marker::PhantomData<S>,
}

impl<'a, S, T> XFrameAllocator<'a, S, T>
where
    S: PrimitiveSize,
    T: PrimitiveRangeManager<Frame<S>, S>,
{
    pub fn new(frame_range_manager: &'a mut T) -> Self {
        Self {
            frame_range_manager,
            _size_marker: core::marker::PhantomData,
        }
    }
}

unsafe impl<'a, T> arch_lib::FrameAllocator<arch_lib::Size4KiB> for XFrameAllocator<'a, Small, T>
where
    T: PrimitiveRangeManager<Frame<Small>, Small>,
{
    fn allocate_frame(&mut self) -> Option<arch_lib::PhysFrame<arch_lib::Size4KiB>> {
        let frame = self.frame_range_manager.allocate_range()?;
        Some(arch_lib::PhysFrame::from_start_address(*frame.start_address()).unwrap())
    }
}

unsafe impl<'a, T> arch_lib::FrameAllocator<arch_lib::Size2MiB> for XFrameAllocator<'a, Medium, T>
where
    T: PrimitiveRangeManager<Frame<Medium>, Medium>,
{
    fn allocate_frame(&mut self) -> Option<arch_lib::PhysFrame<arch_lib::Size2MiB>> {
        let frame = self.frame_range_manager.allocate_range()?;
        Some(arch_lib::PhysFrame::from_start_address(*frame.start_address()).unwrap())
    }
}

unsafe impl<'a, T> arch_lib::FrameAllocator<arch_lib::Size1GiB> for XFrameAllocator<'a, Large, T>
where
    T: PrimitiveRangeManager<Frame<Large>, Large>,
{
    fn allocate_frame(&mut self) -> Option<arch_lib::PhysFrame<arch_lib::Size1GiB>> {
        let frame = self.frame_range_manager.allocate_range()?;
        Some(arch_lib::PhysFrame::from_start_address(*frame.start_address()).unwrap())
    }
}

impl<S> From<arch_lib::MapToError<S>> for MemError
where
    S: arch_lib::PageSize,
{
    fn from(value: arch_lib::MapToError<S>) -> Self {
        match value {
            arch_lib::MapToError::FrameAllocationFailed => MemError::OutOfMemory,
            arch_lib::MapToError::ParentEntryHugePage => {
                MemError::ArchError(ArchError::ParentEntryHugePage)
            }
            arch_lib::MapToError::PageAlreadyMapped(phys_frame) => {
                MemError::AlreadyMapped(PhysAddr::new(phys_frame.start_address().as_u64()))
            }
        }
    }
}

macro_rules! into_arch_page {
    ($x86_64_size: path, $size: path) => {
        impl Into<arch_lib::Page<$x86_64_size>> for Page<$size> {
            fn into(self) -> arch_lib::Page<$x86_64_size> {
                arch_lib::Page::from_start_address(*self.start_address()).unwrap()
            }
        }
    };

    // recurse
    ($x86_64_size: path, $size: path, $($rest_x86_64_size: path, $rest_size: path),*) => {
        into_arch_page!($x86_64_size, $size);
        into_arch_page!($($rest_x86_64_size, $rest_size),*);
    };
}

into_arch_page!(
    arch_lib::Size4KiB,
    Small,
    arch_lib::Size2MiB,
    Medium,
    arch_lib::Size1GiB,
    Large
);

macro_rules! into_arch_frame {
    ($x86_64_size: path, $size: path) => {
        impl Into<arch_lib::PhysFrame<$x86_64_size>> for Frame<$size> {
            fn into(self) -> arch_lib::PhysFrame<$x86_64_size> {
                arch_lib::PhysFrame::from_start_address(*self.start_address()).unwrap()
            }
        }

        impl From<arch_lib::PhysFrame<$x86_64_size>> for Frame<$size> {
            fn from(value: arch_lib::PhysFrame<$x86_64_size>) -> Self {
                Frame::from_start_address(PhysAddr::new(value.start_address().as_u64())).unwrap()
            }
        }
    };

    // recurse
    ($x86_64_size: path, $size: path, $($rest_x86_64_size: path, $rest_size: path),*) => {
        into_arch_frame!($x86_64_size, $size);
        into_arch_frame!($($rest_x86_64_size, $rest_size),*);
    };
}

into_arch_frame!(
    arch_lib::Size4KiB,
    Small,
    arch_lib::Size2MiB,
    Medium,
    arch_lib::Size1GiB,
    Large
);

impl From<MapFlags> for PageTableFlags {
    fn from(value: MapFlags) -> Self {
        let mut flags = Self::PRESENT;
        if value.contains(MapFlags::WRITABLE) {
            flags |= Self::WRITABLE;
        }
        if value.contains(MapFlags::USER_ACCESSIBLE) {
            flags |= Self::USER_ACCESSIBLE;
        }
        if !value.contains(MapFlags::EXECUTABLE) {
            flags |= Self::NO_EXECUTE;
        }
        if value.contains(MapFlags::CACHE_DISABLE) {
            flags |= Self::NO_CACHE;
        }
        flags
    }
}

impl Into<arch_lib::PageTableFlags> for PageTableFlags {
    fn into(self) -> arch_lib::PageTableFlags {
        arch_lib::PageTableFlags::from_bits(self.bits()).unwrap()
    }
}

impl From<PageTableFlags> for MapFlags {
    fn from(value: PageTableFlags) -> Self {
        let mut flags = MapFlags::empty();
        if value.contains(PageTableFlags::WRITABLE) {
            flags |= MapFlags::WRITABLE;
        }
        if value.contains(PageTableFlags::USER_ACCESSIBLE) {
            flags |= MapFlags::USER_ACCESSIBLE;
        }
        if !value.contains(PageTableFlags::NO_EXECUTE) {
            flags |= MapFlags::EXECUTABLE;
        }
        if value.contains(PageTableFlags::NO_CACHE) {
            flags |= MapFlags::CACHE_DISABLE;
        }
        flags
    }
}

impl Into<arch_lib::PageTableIndex> for PageTableIndex {
    fn into(self) -> arch_lib::PageTableIndex {
        arch_lib::PageTableIndex::new(self.value())
    }
}

impl Into<arch_lib::PageTable> for PageTable {
    fn into(self) -> arch_lib::PageTable {
        // SAFETY: Both structs are canonical representations of a page table,
        // and therefore have the same memory layout.
        unsafe { mem::transmute(self) }
    }
}

impl From<arch_lib::PageTable> for PageTable {
    fn from(val: arch_lib::PageTable) -> PageTable {
        // SAFETY: Both structs are canonical representations of a page table,
        // and therefore have the same memory layout.
        unsafe { mem::transmute(val) }
    }
}

impl MemError {
    pub(crate) fn from_unmap_error<S>(error: arch_lib::UnmapError, page: Page<S>) -> Self
    where
        S: PrimitiveSize,
    {
        match error {
            arch_lib::UnmapError::PageNotMapped => MemError::NotMapped(page.into()),
            arch_lib::UnmapError::ParentEntryHugePage => {
                MemError::ArchError(ArchError::ParentEntryHugePage)
            }
            arch_lib::UnmapError::InvalidFrameAddress(addr) => {
                MemError::InvalidFrameAddress(addr.into())
            }
        }
    }
}

impl PageTable {
    /// Converts a &mut PageTable reference to a &mut arch_lib::PageTable reference.
    pub(crate) fn as_arch_mut(&mut self) -> &mut arch_lib::PageTable {
        // SAFETY: Both structs are canonical representations of a page table,
        // and therefore have the same memory layout.
        unsafe { &mut *(self as *mut _ as *mut arch_lib::PageTable) }
    }

    /// Converts a &arch_lib::PageTable reference to a &PageTable reference.
    pub(crate) fn from_arch_ref(arch_table: &arch_lib::PageTable) -> &Self {
        // SAFETY: Both structs are canonical representations of a page table,
        // and therefore have the same memory layout.
        unsafe { &*(arch_table as *const _ as *const Self) }
    }

    /// Converts a &mut arch_lib::PageTable reference to a &mut PageTable reference.
    pub(crate) fn from_arch_mut(arch_table: &mut arch_lib::PageTable) -> &mut Self {
        // SAFETY: Both structs are canonical representations of a page table,
        // and therefore have the same memory layout.
        unsafe { &mut *(arch_table as *mut _ as *mut Self) }
    }
}
