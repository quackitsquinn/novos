//! Address translation trait

use cake::log::trace;

use crate::{
    MapFlags, MemError,
    arch::RecursivePageTable,
    paging::{
        Frame, Large, Medium, MemoryFragment, Page, PageTableIndex, Small, VirtAddr,
        accessor::{self, PagetableAccessor, build_vaddress},
        primitives::{AnyFragment, DirectMapping, FrameClass, PageClass},
    },
};

/// A trait for translating virtual addresses to physical addresses in a given address space.
pub trait Translate {
    /// Translates a virtual address to a physical address in the context of this address space.
    /// Returns a `TranslateResult` indicating the outcome of the translation.
    fn translate(&self, addr: VirtAddr) -> TranslateResult;
}

/// The result of a translation attempt, indicating whether the translation was successful, not mapped, or resulted in an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateResult {
    /// The translation was successful, and the resulting physical address is contained in the `AnyFragment<FrameClass>`.
    Success(DirectMapping, MapFlags),
    /// The virtual address is not mapped in this address space.
    NotMapped,
    /// An error occurred during the translation process, such as an invalid address or a failure to access the page tables.
    Error(MemError),
}

impl From<MemError> for TranslateResult {
    fn from(err: MemError) -> Self {
        TranslateResult::Error(err)
    }
}

impl<T> Translate for T
where
    T: PagetableAccessor,
{
    fn translate(&self, addr: VirtAddr) -> TranslateResult {
        let (l4, l3, l2, l1) = accessor::dissolve_address(addr);
        let l3_table = match self.l3_table(l4) {
            Ok(table) => table,
            Err(e) => return TranslateResult::Error(e),
        };

        let l3_ent = l3_table.read_entry(l3);
        if !l3_ent.is_present() {
            return TranslateResult::NotMapped;
        } else if l3_ent.is_huge() {
            let frame = Frame::from_start_address(l3_ent.addr()).unwrap();
            let page = Page::from_start_address(accessor::build_vaddress(
                l4,
                l3,
                PageTableIndex::MIN,
                PageTableIndex::MIN,
            ))
            .unwrap();
            return TranslateResult::Success(DirectMapping::Large(page, frame), l3_ent.flags());
        }

        let l2_table = match self.l2_table(l4, l3) {
            Ok(table) => table,
            Err(e) => return TranslateResult::Error(e),
        };

        let l2_ent = l2_table.read_entry(l2);
        if !l2_ent.is_present() {
            return TranslateResult::NotMapped;
        } else if l2_ent.is_huge() {
            let frame = Frame::from_start_address(l2_ent.addr()).unwrap();
            let page =
                Page::from_start_address(accessor::build_vaddress(l4, l3, l2, PageTableIndex::MIN))
                    .unwrap();
            return TranslateResult::Success(DirectMapping::Medium(page, frame), l2_ent.flags());
        }

        let l1_table = match self.l1_table(l4, l3, l2) {
            Ok(table) => table,
            Err(e) => return TranslateResult::Error(e),
        };

        let l1_ent = l1_table.read_entry(l1);
        if !l1_ent.is_present() {
            return TranslateResult::NotMapped;
        } else if l1_ent.is_huge() {
            return TranslateResult::Error(MemError::InvalidOperation);
        }

        let frame = Frame::from_start_address(l1_ent.addr()).unwrap();
        let page = Page::from_start_address(accessor::build_vaddress(l4, l3, l2, l1)).unwrap();
        TranslateResult::Success(DirectMapping::Small(page, frame), l1_ent.flags())
    }
}
