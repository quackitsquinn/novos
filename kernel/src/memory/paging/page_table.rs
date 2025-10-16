use core::fmt::Debug;

use x86_64::structures::paging::{
    FrameDeallocator, Mapper, OffsetPageTable, PageTable, PageTableFlags, PageTableIndex,
    PhysFrame, RecursivePageTable, Translate,
    mapper::{
        FlagUpdateError, MapToError, MapperFlush, MapperFlushAll, TranslateError, TranslateResult,
        UnmapError,
    },
};

use crate::memory::paging::{
    KernelPage, KernelPageSize, KernelPhysFrame, builder::PageTableBuilder, page_tree::PageTree,
    phys::FRAME_ALLOCATOR,
};

/// Represents the currently active page table, which can be either the
/// limine page table that is an offset page table or a remapped recursive page table.
pub struct ActivePageTable<'a> {
    limine: Option<OffsetPageTable<'a>>,
    // This will always exist after the switch due to process page tables sharing the same recursive mapping
    remapped: Option<RecursivePageTable<'a>>,
}

macro_rules! active_pt {
    (mut $self: ident.$fn: ident($($arg:tt)*)) => {{
        if let Some(lpt) = &mut $self.limine {
             lpt.$fn($($arg)*)
        } else if let Some(rpt) = &mut $self.remapped {
             rpt.$fn($($arg)*)
        } else {
            panic!("No active page table");
        }
    }};

    ($self: ident.$fn: ident($($arg:tt)*)) => {{
        if let Some(lpt) = &$self.limine {
            lpt.$fn($($arg)*)
        } else if let Some(rpt) = &$self.remapped {
            rpt.$fn($($arg)*)
        } else {
            panic!("No active page table");
        }
    }};
}

impl<'a> ActivePageTable<'a> {
    pub(super) fn new(limine: OffsetPageTable<'a>) -> Self {
        Self {
            limine: Some(limine),
            remapped: None,
        }
    }

    pub fn init_limine(&mut self, opt: OffsetPageTable<'a>) {
        self.limine = Some(opt);
    }

    pub fn switch(&mut self, rpt: RecursivePageTable<'a>) {
        self.limine.take();
        self.remapped = Some(rpt);
    }

    /// Copies the higher half (from PML4 entry 256 and above) from the active page table into the provided builder's page table.
    /// This is useful when creating a new page table that needs to inherit the kernel's higher half mappings. (e.g. for a new process)
    /// Keep in mind that this does not copy the higher half mappings, but
    /// rather sets the pml4 entries to point to the same page tables as the active page table.
    /// This means that changes to the page tables in the active page table will be reflected in the new page table,
    /// and vice versa, until the new page table is modified to create its own mappings.
    ///
    /// All page tables that are loaded are *required* to have the higher half mapped.
    /// Failure to do so will result in undefined behavior.
    pub fn map_kernel_into<T: Iterator<Item = KernelPage>>(
        &self,
        builder: &mut PageTableBuilder<T>,
    ) {
        let pml4 = builder.pml4_mut();

        let kernel_pml4 = if let Some(lpt) = &self.limine {
            lpt.level_4_table()
        } else if let Some(rpt) = &self.remapped {
            rpt.level_4_table()
        } else {
            panic!("No active page table");
        };

        for (i, entry) in kernel_pml4.iter().skip(256).enumerate() {
            pml4[i + 256] = entry.clone();
        }
    }

    pub unsafe fn reclaim_lower_half(&mut self) {
        for i in 0..256 {
            let pml4 = self.get_pml4_mut();
            if pml4[i].is_unused() {
                continue;
            }

            unsafe { self.reclaim([Some(PageTableIndex::new(i as u16)), None, None]) };
        }
    }

    unsafe fn reclaim(&mut self, indexes: [Option<PageTableIndex>; 3]) {
        let level = indexes.iter().filter(|i| i.is_some()).count();

        let mut frame_allocator = FRAME_ALLOCATOR.get();

        for i in 0..512 {
            let table = unsafe { self.get_mut(indexes).expect("Invalid page table indexes") };
            let entry = &mut table[i];
            if entry.is_unused() {
                continue;
            }
            let frame = entry.frame().expect("Entry has no frame");

            if level == 3 {
                unsafe {
                    frame_allocator.deallocate_frame(frame);
                }
                continue;
            }

            let mut new_path = indexes;
            new_path[level] = Some(PageTableIndex::new(i as u16));
            unsafe {
                // Reclaim recursively
                self.reclaim(new_path);
                // Now this frame is free to be deallocated
                frame_allocator.deallocate_frame(frame)
            };
        }
    }
}

impl Mapper<KernelPageSize> for ActivePageTable<'_> {
    fn unmap(
        &mut self,
        page: KernelPage,
    ) -> Result<(PhysFrame<KernelPageSize>, MapperFlush<KernelPageSize>), UnmapError> {
        active_pt!(mut self.unmap(page))
    }

    unsafe fn update_flags(
        &mut self,
        page: KernelPage,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<KernelPageSize>, FlagUpdateError> {
        // SAFETY: held by caller
        unsafe { active_pt!(mut self.update_flags(page, flags)) }
    }

    unsafe fn set_flags_p4_entry(
        &mut self,
        page: KernelPage,
        flags: PageTableFlags,
    ) -> Result<MapperFlushAll, FlagUpdateError> {
        unsafe { active_pt!(mut self.set_flags_p4_entry(page, flags)) }
    }

    unsafe fn set_flags_p3_entry(
        &mut self,
        page: KernelPage,
        flags: PageTableFlags,
    ) -> Result<MapperFlushAll, FlagUpdateError> {
        unsafe { active_pt!(mut self.set_flags_p3_entry(page, flags)) }
    }

    unsafe fn set_flags_p2_entry(
        &mut self,
        page: KernelPage,
        flags: PageTableFlags,
    ) -> Result<MapperFlushAll, FlagUpdateError> {
        unsafe { active_pt!(mut self.set_flags_p2_entry(page, flags)) }
    }

    fn translate_page(&self, page: KernelPage) -> Result<KernelPhysFrame, TranslateError> {
        active_pt!(self.translate_page(page))
    }

    unsafe fn map_to_with_table_flags<A>(
        &mut self,
        page: KernelPage,
        frame: KernelPhysFrame,
        flags: PageTableFlags,
        parent_table_flags: PageTableFlags,
        frame_allocator: &mut A,
    ) -> Result<MapperFlush<KernelPageSize>, MapToError<KernelPageSize>>
    where
        Self: Sized,
        A: x86_64::structures::paging::FrameAllocator<x86_64::structures::paging::Size4KiB>
            + ?Sized,
    {
        // SAFETY: held by caller
        unsafe {
            active_pt!(mut self.map_to_with_table_flags(
                page,
                frame,
                flags,
                parent_table_flags,
                frame_allocator
            ))
        }
    }
}

impl Translate for ActivePageTable<'_> {
    fn translate(&self, addr: x86_64::VirtAddr) -> TranslateResult {
        active_pt!(self.translate(addr))
    }
}

impl Debug for ActivePageTable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelPageTable")
            .field("limine", &self.limine.is_some())
            .field("remapped", &self.remapped.is_some())
            .finish()
    }
}

impl PageTree for ActivePageTable<'_> {
    fn get_l3(
        &self,
        pml4_index: x86_64::structures::paging::PageTableIndex,
    ) -> Option<&x86_64::structures::paging::PageTable> {
        active_pt!(self.get_l3(pml4_index))
    }

    unsafe fn get_l3_mut(
        &mut self,
        pml4_index: x86_64::structures::paging::PageTableIndex,
    ) -> Option<&mut x86_64::structures::paging::PageTable> {
        unsafe { active_pt!(mut self.get_l3_mut(pml4_index)) }
    }

    fn get_l2(
        &self,
        pml4_index: x86_64::structures::paging::PageTableIndex,
        pdpt_index: x86_64::structures::paging::PageTableIndex,
    ) -> Option<&x86_64::structures::paging::PageTable> {
        active_pt!(self.get_l2(pml4_index, pdpt_index))
    }

    unsafe fn get_l2_mut(
        &mut self,
        pml4_index: x86_64::structures::paging::PageTableIndex,
        pdpt_index: x86_64::structures::paging::PageTableIndex,
    ) -> Option<&mut x86_64::structures::paging::PageTable> {
        unsafe { active_pt!(mut self.get_l2_mut(pml4_index, pdpt_index)) }
    }

    fn get_l1(
        &self,
        pml4_index: x86_64::structures::paging::PageTableIndex,
        pdpt_index: x86_64::structures::paging::PageTableIndex,
        pd_index: x86_64::structures::paging::PageTableIndex,
    ) -> Option<&x86_64::structures::paging::PageTable> {
        active_pt!(self.get_l1(pml4_index, pdpt_index, pd_index))
    }

    unsafe fn get_l1_mut(
        &mut self,
        pml4_index: x86_64::structures::paging::PageTableIndex,
        pdpt_index: x86_64::structures::paging::PageTableIndex,
        pd_index: x86_64::structures::paging::PageTableIndex,
    ) -> Option<&mut x86_64::structures::paging::PageTable> {
        unsafe { active_pt!(mut self.get_l1_mut(pml4_index, pdpt_index, pd_index)) }
    }

    fn get_pml4(&self) -> &PageTable {
        active_pt!(self.get_pml4())
    }

    fn get_pml4_mut(&mut self) -> &mut PageTable {
        active_pt!(mut self.get_pml4_mut())
    }
}
