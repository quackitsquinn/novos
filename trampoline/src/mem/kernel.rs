use core::panic;

use cake::{info, trace};
use kelp::Elf;
use kvmm::{KernelPage, KernelPhysFrame, pagetable::PagetableBuilder};
use x86_64::{
    VirtAddr,
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, page::PageRange},
};

use crate::{
    arch::{KERNEL_JUMP_LOAD_POINT, copy_jump_point},
    mem::{MAPPER, PAGETABLE},
    requests::KERNEL_FILE,
};

pub struct MappedPageIterator {
    iter: PageRange,
}

impl MappedPageIterator {
    pub fn new(start: KernelPage, end: KernelPage) -> Self {
        MappedPageIterator {
            iter: KernelPage::range(start, end),
        }
    }

    pub fn from_page_range(range: PageRange) -> Self {
        MappedPageIterator { iter: range }
    }

    pub fn next_page(&mut self) -> KernelPage {
        self.iter.next().expect("No more pages available")
    }
}

impl Iterator for MappedPageIterator {
    type Item = (KernelPage, KernelPhysFrame);

    fn next(&mut self) -> Option<Self::Item> {
        let mut mapper = MAPPER.get().expect("uninit").lock();
        let mut table = PAGETABLE.get().expect("uninit").lock();
        let page = self.iter.next()?;
        let frame = mapper.next_frame()?;
        unsafe {
            table.map_to(
                page,
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                &mut *mapper,
            )
        }
        .expect("Failed to map page")
        .flush();
        Some((page, frame))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kernel {
    /// The physical frame that corresponds to the kernel's PML4.
    pub cr3: PhysFrame,
    /// The virtual address of the kernel's entry point.
    pub rip: VirtAddr,
    /// The virtual address of the kernel's stack pointer.
    pub rsp: VirtAddr,
}

impl Kernel {
    /// Creates a new kernel instance with the given CR3, RIP, and RSP.
    ///
    /// # Safety
    /// The caller must ensure that all addresses are valid and properly aligned.
    pub unsafe fn new(cr3: PhysFrame, rip: VirtAddr, rsp: VirtAddr) -> Self {
        Kernel { cr3, rip, rsp }
    }
}
