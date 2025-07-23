use core::panic;

use cake::trace;
use kelp::Elf;
use kvmm::{KernelPage, KernelPhysFrame, pagetable::PagetableBuilder};
use x86_64::{
    VirtAddr,
    registers::model_specific::LStar,
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, page::PageRange},
};

use crate::{
    arch::{KERNEL_JUMP_LOAD_POINT, copy_jump_point},
    mem::{MAPPER, PAGETABLE},
    requests::KERNEL_FILE,
};

const ENTRY_PONT_NAME: &str = "_start";
const STACK_SIZE: usize = 0x1_000_000; // Example stack size, adjust as needed
const STACK_BASE: VirtAddr = VirtAddr::new_truncate(0x800_000_000_000);

pub struct MappedPageIterator {
    iter: PageRange,
}

impl MappedPageIterator {
    pub fn new(start: KernelPage, end: KernelPage) -> Self {
        MappedPageIterator {
            iter: KernelPage::range(start, end),
        }
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

/// Maps the kernel and returns the physical frame that corresponds to the kernel's PML4 and the virtua
pub fn map_kernel() -> Kernel {
    let mut iter = MappedPageIterator::new(
        KernelPage::containing_address(VirtAddr::new(0x6ff_fff_fff_fff)), // Example start address
        KernelPage::containing_address(VirtAddr::new(0x7ff_fff_fff_fff)), // Example end address
    );

    let mut builder = PagetableBuilder::new(&mut iter);

    let kernel = KERNEL_FILE.get().expect("Kernel file not found");

    let elf = Elf::new(kernel).expect("Failed to parse ELF file");

    for segment in elf.segments() {
        if segment.p_align > 4096 {
            panic!("Segment alignment is too large: {}", segment.p_align);
        }

        let p_vaddr = VirtAddr::new(segment.p_vaddr);
        let mut virt_range = Page::range_inclusive(
            KernelPage::containing_address(p_vaddr),
            KernelPage::containing_address(p_vaddr + segment.p_memsz as u64 - 1),
        );
        let offset = segment.p_offset as usize;
        let kernel_slice = kernel[offset..offset + segment.p_filesz as usize]
            .chunks(4096)
            .map(|c| (c, virt_range.next().expect("virt range exhausted")));

        let flags = segment_to_pt(segment.p_flags);

        for (data, dest_page) in kernel_slice {
            let (copy_page, dest_frame) = builder.next_page();
            unsafe {
                copy_page
                    .start_address()
                    .as_mut_ptr::<u8>()
                    .copy_from_nonoverlapping(data.as_ptr(), data.len())
            };
            builder.map_page(dest_page, dest_frame, flags);
        }
    }

    let (jump_page, jump_frame) = builder.next_page();

    unsafe {
        copy_jump_point(jump_page);
    }

    builder.map_page(
        KERNEL_JUMP_LOAD_POINT,
        jump_frame,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    let stack_page_range = KernelPage::range_inclusive(
        KernelPage::containing_address(STACK_BASE),
        KernelPage::containing_address(VirtAddr::new_truncate(
            STACK_BASE.as_u64() + STACK_SIZE as u64 - 1,
        )),
    );

    for page in stack_page_range {
        let mut mapper = MAPPER.get().expect("uninit").lock();
        let frame = mapper.next_frame().expect("No frames available");
        drop(mapper);
        builder.map_page(
            page,
            frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
        );
    }

    // Build the pagetable. We don't need to worry about releasing the resources since the resources will be disposed
    // as soon as the kernel is loaded.
    let (pgtbl, frame, _) = builder.build_and_release(None);

    let mut rip = None;
    // Now iterate over the symbols in the ELF file to find the entry point.
    for sym in elf.symbols().expect("Unable to find symbol table") {
        if !sym.is_function() {
            continue;
        }

        let name = unsafe {
            elf.strings()
                .expect("Unable to find string table")
                .get_str(sym.st_name as usize)
                .expect("Invalid symbol name")
        };

        trace!("Found symbol: {} at {:#x}", name, sym.st_value);

        if name == ENTRY_PONT_NAME {
            rip = Some(VirtAddr::new(sym.st_value));
        }
    }
    // We don't want drop code to run on the pagetable, so we convert it into a raw pointer.
    let _ = unsafe { pgtbl.into_raw() };

    unsafe {
        Kernel::new(
            frame,
            rip.expect("Kernel entry point not found!"),
            VirtAddr::new(STACK_BASE.as_u64() + STACK_SIZE as u64 - 8),
        )
    }
}

fn segment_to_pt(segment_flags: u32) -> PageTableFlags {
    let mut flags = PageTableFlags::empty();
    if segment_flags & 0x1 == 0 {
        flags |= PageTableFlags::NO_EXECUTE;
    }
    if segment_flags & 0x2 != 0 {
        flags |= PageTableFlags::WRITABLE;
    }
    if segment_flags & 0x4 != 0 {
        flags |= PageTableFlags::PRESENT
    }
    flags
}
