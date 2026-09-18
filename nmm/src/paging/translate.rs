//! Address translation trait

use crate::{
    MemError,
    arch::RecursivePageTable,
    paging::{
        Frame, Large, Medium, MemoryFragment, Page, PageTableIndex, Small, VirtAddr,
        accessor::{self, PagetableAccessor, build_vaddress},
        primitives::{AnyFragment, FrameClass, PageClass},
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
    Success(PageMapping),
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
            return TranslateResult::Success(PageMapping::Large(
                Page::from_start_address(accessor::build_vaddress(
                    l4,
                    l3,
                    PageTableIndex::MIN,
                    PageTableIndex::MIN,
                ))
                .unwrap(),
                frame,
            ));
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
            return TranslateResult::Success(PageMapping::Medium(
                Page::from_start_address(accessor::build_vaddress(l4, l3, l2, PageTableIndex::MIN))
                    .unwrap(),
                frame,
            ));
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
        TranslateResult::Success(PageMapping::Small(
            Page::from_start_address(accessor::build_vaddress(l4, l3, l2, l1)).unwrap(),
            frame,
        ))
    }
}

/// A direct mapping between a page and a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMapping {
    /// A mapping between a small page and a small frame.
    Small(Page<Small>, Frame<Small>),
    /// A mapping between a medium page and a medium frame.
    Medium(Page<Medium>, Frame<Medium>),
    /// A mapping between a large page and a large frame.
    Large(Page<Large>, Frame<Large>),
}

impl PageMapping {
    /// Returns the page associated with this mapping.
    pub fn page(&self) -> AnyFragment<PageClass> {
        match self {
            PageMapping::Small(page, _) => AnyFragment::Small(*page),
            PageMapping::Medium(page, _) => AnyFragment::Medium(*page),
            PageMapping::Large(page, _) => AnyFragment::Large(*page),
        }
    }

    /// Returns the frame associated with this mapping.
    pub fn frame(&self) -> AnyFragment<FrameClass> {
        match self {
            PageMapping::Small(_, frame) => AnyFragment::Small(*frame),
            PageMapping::Medium(_, frame) => AnyFragment::Medium(*frame),
            PageMapping::Large(_, frame) => AnyFragment::Large(*frame),
        }
    }
}
