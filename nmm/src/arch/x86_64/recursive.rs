//! A deeply cursed solution to a deeply cursed problem. (kinda)
//!
//! This module acts mostly as a hack to get proper rust-analyzer support for x86_64::RecursivePageTable, plus as a way to somewhat normalize what the cross platform
//! Mapping system will look like. Again, deeply cursed but there IS a not completely cursed reason for it.use std::marker;

use core::fmt::Debug;

use x86_64::structures::paging::mapper::Mapper as _;

use crate::{
    MapFlags, MemError,
    arch::x86_64::{PageTableFlags, XFrameAllocator},
    paging::{
        FragmentManager, Frame, Large, Medium, Page, PageTable, PageTableIndex, Small,
        map::{Flush, MemoryMapper},
    },
};

mod arch_lib {
    pub use x86_64::structures::paging::PageTable;

    #[cfg(target_arch = "x86_64")]
    pub use x86_64::structures::paging::RecursivePageTable as XRecursive;

    #[cfg(not(target_arch = "x86_64"))]
    mod recursive_spoof {
        use x86_64::structures::paging::{
            FrameAllocator, Mapper, Page, PageSize, PageTable, PageTableFlags, PageTableIndex,
            PhysFrame, Size4KiB,
            mapper::{
                FlagUpdateError, MapToError, MapperFlush, MapperFlushAll, TranslateError,
                UnmapError,
            },
        };

        pub struct RecursivePageTable<'a> {
            _phantom: core::marker::PhantomData<&'a ()>,
        }

        impl<'a> RecursivePageTable<'a> {
            pub unsafe fn new_unchecked(
                _table: &'a mut PageTable,
                _recursive_index: PageTableIndex,
            ) -> Self {
                unimplemented!("Recursive page tables are only supported on x86_64 architecture");
            }

            pub fn level_4_table(&self) -> &super::PageTable {
                unimplemented!("Recursive page tables are only supported on x86_64 architecture");
            }

            pub fn level_4_table_mut(&mut self) -> &mut super::PageTable {
                unimplemented!("Recursive page tables are only supported on x86_64 architecture");
            }
        }

        impl<'a, S> Mapper<S> for RecursivePageTable<'a>
        where
            S: PageSize,
        {
            unsafe fn map_to_with_table_flags<A>(
                &mut self,
                _page: Page<S>,
                _frame: PhysFrame<S>,
                _flags: PageTableFlags,
                _parent_table_flags: PageTableFlags,
                _frame_allocator: &mut A,
            ) -> Result<MapperFlush<S>, MapToError<S>>
            where
                Self: Sized,
                A: FrameAllocator<Size4KiB> + ?Sized,
            {
                todo!()
            }

            fn unmap(
                &mut self,
                _page: Page<S>,
            ) -> Result<(PhysFrame<S>, MapperFlush<S>), UnmapError> {
                todo!()
            }

            unsafe fn update_flags(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlush<S>, FlagUpdateError> {
                todo!()
            }

            unsafe fn set_flags_p4_entry(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlushAll, FlagUpdateError> {
                todo!()
            }

            unsafe fn set_flags_p3_entry(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlushAll, FlagUpdateError> {
                todo!()
            }

            unsafe fn set_flags_p2_entry(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlushAll, FlagUpdateError> {
                todo!()
            }

            fn translate_page(&self, _page: Page<S>) -> Result<PhysFrame<S>, TranslateError> {
                todo!()
            }
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    pub use recursive_spoof::RecursivePageTable as XRecursive;
}

/// A x86_64 specific implementation of a recursive page table, wrapping the x86_64's crate implementation of a recursive page table.
pub struct RecursivePageTable<'a>(arch_lib::XRecursive<'a>, PageTableIndex);

impl<'a> RecursivePageTable<'a> {
    /// Creates a new RecursivePageTable from a mutable reference to the level 4 page table and the recursive index.
    /// # Safety
    /// The caller must ensure that the provided page table is the actual level 4 page table, and that the recursive index is correctly set up in the page tables.
    pub unsafe fn new(table: &'a mut PageTable, recursive_index: PageTableIndex) -> Self {
        let table = unsafe {
            arch_lib::XRecursive::new_unchecked(table.as_arch_mut(), recursive_index.into())
        };
        Self(table, recursive_index)
    }
    /// Returns the recursive index used for this recursive page table.
    pub fn recursive_index(&self) -> PageTableIndex {
        self.1
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &arch_lib::PageTable {
        let p4 = self.0.level_4_table();
        p4
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut arch_lib::PageTable {
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
        A: FragmentManager<Frame<Small>, Small>,
    {
        let mut x_fa = XFrameAllocator::new(allocator);
        let flags: PageTableFlags = flags.into();

        unsafe {
            let _ = self
                .0
                .map_to(page.into(), frame.into(), flags.into(), &mut x_fa)?;
        };

        Ok(unsafe { Flush::flush_page(page) })
    }

    unsafe fn unmap(
        &mut self,
        page: crate::paging::Page<Small>,
    ) -> Result<(Frame<Small>, Flush), MemError> {
        let result = self.0.unmap(page.into());
        match result {
            Ok((frame, _)) => Ok((frame.into(), unsafe { Flush::flush_page(page) })),
            Err(e) => Err(MemError::from_unmap_error(e, page)),
        }
    }
}

impl Debug for RecursivePageTable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RecursivePageTable")
            .field("recursive_index", &self.recursive_index())
            .finish()
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
                A: FragmentManager<Frame<Small>, Small>,
            {
                let mut x_fa = XFrameAllocator::new(allocator);
                let mut flags: PageTableFlags = flags.into();
                flags.insert(PageTableFlags::HUGE_PAGE);

                unsafe {
                    let _ = self
                        .0
                        .map_to(page.into(), frame.into(), flags.into(), &mut x_fa)?;
                };

                Ok(unsafe { Flush::flush_page(page) })
            }

            unsafe fn unmap(
                &mut self,
                page: crate::paging::Page<$size>,
            ) -> Result<(Frame<$size>, Flush), MemError> {
                let result = self.0.unmap(page.into());
                match result {
                    Ok((frame, _)) => Ok((frame.into(), unsafe { Flush::flush_page(page) })),
                    Err(e) => Err(MemError::from_unmap_error(e, page)),
                }
            }
        }
    };
}

impl_memory_mapper_huge!(Medium);
impl_memory_mapper_huge!(Large);
