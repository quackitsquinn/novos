//! A deeply cursed solution to a deeply cursed problem. (kinda)
//!
//! This module acts mostly as a hack to get proper rust-analyzer support for x86_64::RecursivePageTable, plus as a way to somewhat normalize what the cross platform
//! Mapping system will look like. Again, deeply cursed but there IS a not completely cursed reason for it.use std::marker;

use x86_64::structures::paging::{PageTable, mapper::RecursivePageTable as XRecursive};

use crate::paging::{Frame, PageTableIndex, PrimitiveRangeManager, Small};

/// A x86_64 specific implementation of a recursive page table, wrapping the x86_64's crate implementation of a recursive page table.
pub struct RecursivePageTable<'a>(XRecursive<'a>, PageTableIndex);

impl<'a> RecursivePageTable<'a> {
    /// Creates a new RecursivePageTable from a mutable reference to the level 4 page table and the recursive index.
    /// # Safety
    /// The caller must ensure that the provided page table is the actual level 4 page table, and that the recursive index is correctly set up in the page tables.
    pub unsafe fn new(table: &'a mut PageTable, recursive_index: PageTableIndex) -> Self {
        let table = unsafe { XRecursive::new_unchecked(table, recursive_index.into()) };
        Self(table, recursive_index)
    }
    /// Returns the recursive index used for this recursive page table.
    pub fn recursive_index(&self) -> PageTableIndex {
        self.1
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &PageTable {
        let p4 = self.0.level_4_table();
        p4
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut PageTable {
        let p4 = self.0.level_4_table_mut();
        p4
    }
}
