use crate::{
    MapFlags, MemError,
    arch::{
        self, PhysAddr,
        x86_64::{ArchError, PageTableFlags},
    },
    paging::{
        Frame, Large, Medium, Page, PageTableIndex, PrimitiveRangeManager, PrimitiveSize, Small,
    },
};

mod arch_lib {
    pub use x86_64::structures::paging::{
        FrameAllocator, Page, PageSize, PageTableFlags, PageTableIndex, PhysFrame, Size1GiB,
        Size2MiB, Size4KiB, mapper::MapToError,
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

impl Into<arch_lib::PageTableFlags> for MapFlags {
    fn into(self) -> arch_lib::PageTableFlags {
        let mut flags = arch_lib::PageTableFlags::PRESENT;
        if self.contains(MapFlags::WRITABLE) {
            flags |= arch_lib::PageTableFlags::WRITABLE;
        }
        if self.contains(MapFlags::USER_ACCESSIBLE) {
            flags |= arch_lib::PageTableFlags::USER_ACCESSIBLE;
        }
        if !self.contains(MapFlags::EXECUTABLE) {
            flags |= arch_lib::PageTableFlags::NO_EXECUTE;
        }
        if self.contains(MapFlags::CACHE_DISABLE) {
            flags |= arch_lib::PageTableFlags::NO_CACHE;
        }
        flags
    }
}

impl Into<arch_lib::PageTableIndex> for PageTableIndex {
    fn into(self) -> arch_lib::PageTableIndex {
        arch_lib::PageTableIndex::new(self.value())
    }
}
