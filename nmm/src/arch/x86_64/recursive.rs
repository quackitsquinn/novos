//! A deeply cursed solution to a deeply cursed problem. (kinda)
//!
//! This module acts mostly as a hack to get proper rust-analyzer support for x86_64::RecursivePageTable, plus as a way to somewhat normalize what the cross platform
//! Mapping system will look like. Again, deeply cursed but there IS a not completely cursed reason for it.use std::marker;

use core::{fmt::Debug, mem::transmute};

use x86_64::structures::paging::mapper::{Mapper as _, Translate as _};

use crate::{
    MapFlags, MemError,
    arch::x86_64::{PageTableFlags, XFrameAllocator, impl_memory_mapper_for},
    paging::{
        EntryMappingFlags, FragmentManager, Frame, Large, Medium, Page, PageTable, PageTableIndex,
        Small,
        map::{Flush, SizedMemoryMapper, Unmapped},
    },
};

mod arch_lib {
    pub use x86_64::structures::paging::PageTable;
    pub use x86_64::structures::paging::mapper::TranslateResult;

    #[cfg(target_arch = "x86_64")]
    pub use x86_64::structures::paging::RecursivePageTable as XRecursive;

    #[cfg(not(target_arch = "x86_64"))]
    mod recursive_spoof {
        use x86_64::structures::paging::{
            FrameAllocator, Mapper, Page, PageSize, PageTable, PageTableFlags, PageTableIndex,
            PhysFrame, Size4KiB, Translate,
            mapper::{
                FlagUpdateError, MapToError, MapperFlush, MapperFlushAll, TranslateError,
                TranslateResult, UnmapError,
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
                unimplemented!()
            }

            fn unmap(
                &mut self,
                _page: Page<S>,
            ) -> Result<(PhysFrame<S>, MapperFlush<S>), UnmapError> {
                unimplemented!()
            }

            unsafe fn update_flags(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlush<S>, FlagUpdateError> {
                unimplemented!()
            }

            unsafe fn set_flags_p4_entry(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlushAll, FlagUpdateError> {
                unimplemented!()
            }

            unsafe fn set_flags_p3_entry(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlushAll, FlagUpdateError> {
                unimplemented!()
            }

            unsafe fn set_flags_p2_entry(
                &mut self,
                _page: Page<S>,
                _flags: PageTableFlags,
            ) -> Result<MapperFlushAll, FlagUpdateError> {
                unimplemented!()
            }

            fn translate_page(&self, _page: Page<S>) -> Result<PhysFrame<S>, TranslateError> {
                unimplemented!()
            }
        }

        impl Translate for RecursivePageTable<'_> {
            fn translate(&self, _addr: x86_64::VirtAddr) -> TranslateResult {
                unimplemented!()
            }
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    pub use recursive_spoof::RecursivePageTable as XRecursive;
}

/// A x86_64 specific implementation of a recursive page table, wrapping the x86_64's crate implementation of a recursive page table.
pub struct RecursivePageTable<'a> {
    inner: arch_lib::XRecursive<'a>,
    recursive_index: PageTableIndex,
}

impl<'a> RecursivePageTable<'a> {
    /// Creates a new RecursivePageTable from a mutable reference to the level 4 page table and the recursive index.
    /// # Safety
    /// The caller must ensure that the provided page table is the actual level 4 page table, and that the recursive index is correctly set up in the page tables.
    pub unsafe fn new(table: &'a mut PageTable, recursive_index: PageTableIndex) -> Self {
        let table = unsafe {
            arch_lib::XRecursive::new_unchecked(table.as_arch_mut(), recursive_index.into())
        };
        Self {
            inner: table,
            recursive_index,
        }
    }
    /// Returns the recursive index used for this recursive page table.
    pub fn recursive_index(&self) -> PageTableIndex {
        self.recursive_index
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &PageTable {
        let p4 = self.inner.level_4_table();
        PageTable::from_arch_ref(p4)
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut PageTable {
        let p4 = self.inner.level_4_table_mut();
        PageTable::from_arch_mut(p4)
    }
}

impl Debug for RecursivePageTable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RecursivePageTable")
            .field("recursive_index", &self.recursive_index())
            .finish()
    }
}

impl_memory_mapper_for!(RecursivePageTable<'_>, Small, false);
impl_memory_mapper_for!(RecursivePageTable<'_>, Medium, true);
impl_memory_mapper_for!(RecursivePageTable<'_>, Large, true);
