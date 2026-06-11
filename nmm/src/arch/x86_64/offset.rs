use core::fmt::Debug;

use x86_64::structures::paging::Mapper as _;

use crate::{
    MapFlags, MemError,
    arch::x86_64::{PageTableFlags, XFrameAllocator},
    paging::{
        Frame, Large, Medium, Page, PageTable, PrimitiveRangeManager, Small, VirtAddr,
        map::{Flush, MemoryMapper},
    },
};

mod arch_lib {
    pub use x86_64::structures::paging::OffsetPageTable;
}

/// An offset page table mapper for x86_64. This mapper uses a fixed offset to access the page tables, and is the most basic type of mapper.
// We wrap x86_64's OffsetPageTable to prevent having x86_64 effect the internal implementation details of the mapper.
pub struct OffsetPageTable<'a> {
    opt: arch_lib::OffsetPageTable<'a>,
}

impl<'a> OffsetPageTable<'a> {
    /// Creates a new RecursivePageTable from a mutable reference to the level 4 page table and the recursive index.
    /// # Safety
    /// The caller must ensure that the provided page table is the actual level 4 page table, and that the recursive index is correctly set up in the page tables.
    pub unsafe fn new(table: &'a mut PageTable, offset: VirtAddr) -> Self {
        let table = unsafe { arch_lib::OffsetPageTable::new(table.as_arch_mut(), *offset) };
        Self { opt: table }
    }
    /// Returns the recursive index used for this recursive page table.
    pub fn phys_offset(&self) -> VirtAddr {
        self.opt.phys_offset().into()
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &PageTable {
        // Just as a programmer's note: I know p4 is x86_64 specific, but it's nicer and gets the point across quicker than level_4_table.
        // If another arch does commonly use more than 4 levels of page tables, then this function is named dumb. but that doesn't matter right now.
        PageTable::from_arch_ref(self.opt.level_4_table())
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut PageTable {
        PageTable::from_arch_mut(self.opt.level_4_table_mut())
    }
}

impl Debug for OffsetPageTable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OffsetPageTable")
            .field("phys_offset", &self.phys_offset())
            .finish()
    }
}

impl MemoryMapper<Small> for OffsetPageTable<'_> {
    fn map<A>(
        &mut self,
        page: Page<Small>,
        frame: Frame<Small>,
        flags: MapFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: PrimitiveRangeManager<Frame<Small>, Small>,
    {
        let mut x_fa = XFrameAllocator::new(allocator);
        let flags: PageTableFlags = flags.into();

        unsafe {
            let _ = self
                .opt
                .map_to(page.into(), frame.into(), flags.into(), &mut x_fa)?;
        };

        Ok(unsafe { Flush::new(page.start_address()) })
    }

    unsafe fn unmap(
        &mut self,
        page: crate::paging::Page<Small>,
    ) -> Result<(Frame<Small>, Flush), MemError> {
        let result = self.opt.unmap(page.into());
        match result {
            Ok((frame, _)) => Ok((frame.into(), unsafe { Flush::new(page.start_address()) })),
            Err(e) => Err(MemError::from_unmap_error(e, page)),
        }
    }
}

macro_rules! impl_memory_mapper_huge {
    ($size:ident) => {
        impl MemoryMapper<$size> for OffsetPageTable<'_> {
            fn map<A>(
                &mut self,
                page: Page<$size>,
                frame: Frame<$size>,
                flags: MapFlags,
                allocator: &mut A,
            ) -> Result<Flush, MemError>
            where
                A: PrimitiveRangeManager<Frame<Small>, Small>,
            {
                let mut x_fa = XFrameAllocator::new(allocator);
                let mut flags: PageTableFlags = flags.into();
                flags.insert(PageTableFlags::HUGE_PAGE);

                unsafe {
                    let _ = self
                        .opt
                        .map_to(page.into(), frame.into(), flags.into(), &mut x_fa)?;
                };

                Ok(unsafe { Flush::new(page.start_address()) })
            }

            unsafe fn unmap(
                &mut self,
                page: crate::paging::Page<$size>,
            ) -> Result<(Frame<$size>, Flush), MemError> {
                let result = self.opt.unmap(page.into());
                match result {
                    Ok((frame, _)) => {
                        Ok((frame.into(), unsafe { Flush::new(page.start_address()) }))
                    }
                    Err(e) => Err(MemError::from_unmap_error(e, page)),
                }
            }
        }
    };
}

impl_memory_mapper_huge!(Medium);
impl_memory_mapper_huge!(Large);
