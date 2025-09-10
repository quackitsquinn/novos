use x86_64::structures::paging::{
    mapper::{
        FlagUpdateError, MapToError, MapperFlush, MapperFlushAll, TranslateError, TranslateResult,
        UnmapError,
    },
    Mapper, OffsetPageTable, Page, PageSize, PageTableFlags, PhysFrame, RecursivePageTable,
    Size4KiB, Translate,
};

use crate::memory::paging::{KernelPage, KernelPageSize, KernelPhysFrame};

pub struct KernelPageTable<'a> {
    limine: Option<OffsetPageTable<'a>>,
    remapped: Option<RecursivePageTable<'a>>,
}

impl<'a> KernelPageTable<'a> {
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
}

macro_rules! active_pt {
    (mut $self: ident.$fn: ident($($arg:tt)*)) => {{
        if let Some(lpt) = &mut $self.limine {
            return lpt.$fn($($arg)*);
        }

        if let Some(rpt) = &mut $self.remapped {
            return rpt.$fn($($arg)*);
        }

        panic!("No active page table");
    }};

    ($self: ident.$fn: ident($($arg:tt)*)) => {{
        if let Some(lpt) = &$self.limine {
            return lpt.$fn($($arg)*);
        }

        if let Some(rpt) = &$self.remapped {
            return rpt.$fn($($arg)*);
        }

        panic!("No active page table");
    }};
}

impl Mapper<KernelPageSize> for KernelPageTable<'_> {
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

impl Translate for KernelPageTable<'_> {
    fn translate(&self, addr: x86_64::VirtAddr) -> TranslateResult {
        active_pt!(self.translate(addr))
    }
}
