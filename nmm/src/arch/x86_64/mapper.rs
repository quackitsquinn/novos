use crate::{
    MapFlags, MemError,
    arch::{
        VirtAddr,
        x86_64::{offset::OffsetPageTable, recursive::RecursivePageTable},
    },
    paging::{
        EntryMappingFlags, FragmentManager, FragmentSize, Frame, Page, PageTable, PageTableIndex,
        Small,
        map::{Flush, MemoryMapper, Unmapped},
    },
};

/// The lowest level of mapper for x86_64.
#[derive(Debug)]
pub enum Mapper {
    /// An offset page table mapper for x86_64. This mapper uses a fixed offset to access the page tables, and is the most basic type of mapper.
    Offset(OffsetPageTable<'static>),
    /// A recursive page table mapper for x86_64. This type of mapper uses a recursive page table mapping to allow for more flexible mapping operations.
    Recursive(RecursivePageTable<'static>),
}

impl Mapper {
    /// Creates a new offset mapper. This is the most basic type of mapper, and is used for the initial bootstrap of the virtual memory system.
    ///
    /// # Safety
    /// The caller must ensure that the provided `root` page table is valid and that the `offset` is correct for the current virtual memory mapping.
    pub unsafe fn new_offset(root: &'static mut PageTable, offset: VirtAddr) -> Self {
        Self::Offset(unsafe { OffsetPageTable::new(root, offset) })
    }

    /// Creates a new recursive mapper. This type of mapper uses a recursive page table mapping to allow for more flexible mapping operations.
    ///
    /// # Safety
    /// The caller must ensure that the provided `root` page table is valid and that the recursive mapping is set up correctly.
    pub unsafe fn new_recursive(
        root: &'static mut PageTable,
        recursive_index: PageTableIndex,
    ) -> Self {
        // TODO: Maybe make sure the recursive index is actually recursive?
        Self::Recursive(unsafe { RecursivePageTable::new(root, recursive_index) })
    }

    pub fn root_table(&self) -> &PageTable {
        match self {
            Mapper::Offset(mapper) => mapper.p4(),
            Mapper::Recursive(mapper) => mapper.p4(),
        }
    }
}

impl<S> MemoryMapper<S> for Mapper
where
    S: FragmentSize,
    RecursivePageTable<'static>: MemoryMapper<S>,
    OffsetPageTable<'static>: MemoryMapper<S>,
{
    fn map<A>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: MapFlags,
        mapping_flags: EntryMappingFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: FragmentManager<Frame<Small>, Small>,
    {
        match self {
            Mapper::Offset(mapper) => mapper.map(page, frame, flags, mapping_flags, allocator),
            Mapper::Recursive(mapper) => mapper.map(page, frame, flags, mapping_flags, allocator),
        }
    }

    unsafe fn unmap(&mut self, page: crate::paging::Page<S>) -> Result<Unmapped<S>, MemError> {
        match self {
            Mapper::Offset(mapper) => unsafe { mapper.unmap(page) },
            Mapper::Recursive(mapper) => unsafe { mapper.unmap(page) },
        }
    }
}
