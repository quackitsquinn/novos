//! A deeply cursed solution to a deeply cursed problem. (kinda)
//!
//! This module acts mostly as a hack to get proper rust-analyzer support for x86_64::RecursivePageTable, plus as a way to somewhat normalize what the cross platform
//! Mapping system will look like. Again, deeply cursed but there IS a not completely cursed reason for it.use std::marker;

mod arch_crate {
    pub use x86_64::structures::paging::{
        PageSize, PageTable,
        mapper::{MapToError, Mapper, RecursivePageTable as XRecursive, UnmapError},
    };
}

use x86_64::structures::paging::mapper::Mapper as _;

use crate::{
    MapFlags, MemError,
    arch::{
        self, ArchError,
        x86_64::{PageTableFlags, XFrameAllocator},
    },
    paging::{
        Frame, Large, Medium, Page, PageTableIndex, PrimitiveRangeManager, Small, UnsizedFrame,
        map::{Flush, MemoryMapper},
        page::UnsizedPage,
    },
};

/// A x86_64 specific implementation of a recursive page table, wrapping the x86_64's crate implementation of a recursive page table.
pub struct RecursivePageTable<'a>(arch_crate::XRecursive<'a>, PageTableIndex);

impl<'a> RecursivePageTable<'a> {
    /// Creates a new RecursivePageTable from a mutable reference to the level 4 page table and the recursive index.
    /// # Safety
    /// The caller must ensure that the provided page table is the actual level 4 page table, and that the recursive index is correctly set up in the page tables.
    pub unsafe fn new(
        table: &'a mut arch_crate::PageTable,
        recursive_index: PageTableIndex,
    ) -> Self {
        let table = unsafe { arch_crate::XRecursive::new_unchecked(table, recursive_index.into()) };
        Self(table, recursive_index)
    }
    /// Returns the recursive index used for this recursive page table.
    pub fn recursive_index(&self) -> PageTableIndex {
        self.1
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &arch_crate::PageTable {
        let p4 = self.0.level_4_table();
        p4
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut arch_crate::PageTable {
        let p4 = self.0.level_4_table_mut();
        p4
    }
}

impl MemoryMapper<Small> for RecursivePageTable<'_> {
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
            self.0
                .map_to(page.into(), frame.into(), flags.into(), &mut x_fa)?
        };

        Ok(unsafe { Flush::new(page.start_address()) })
    }

    unsafe fn unmap(
        &mut self,
        page: crate::paging::Page<Small>,
    ) -> Result<(Frame<Small>, Flush), MemError> {
        let result = self.0.unmap(page.into());
        match result {
            Ok((frame, _)) => Ok((frame.into(), unsafe { Flush::new(page.start_address()) })),
            Err(e) => Err(MemError::from_unmap_error(e, page)),
        }
    }
}

macro_rules! impl_memory_mapper_huge {
    ($size:ident) => {
        impl MemoryMapper<$size> for RecursivePageTable<'_> {
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
                    self.0
                        .map_to(page.into(), frame.into(), flags.into(), &mut x_fa)?
                };

                Ok(unsafe { Flush::new(page.start_address()) })
            }

            unsafe fn unmap(
                &mut self,
                page: crate::paging::Page<$size>,
            ) -> Result<(Frame<$size>, Flush), MemError> {
                let result = self.0.unmap(page.into());
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
