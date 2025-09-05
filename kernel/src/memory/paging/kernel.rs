use core::{convert::Infallible, iter};

use goblin::elf64::program_header::ProgramHeader;
use log::{debug, info};
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        page::PageRange, page_table::PageTableEntry, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PageTableIndex, PhysFrame, RecursivePageTable,
    },
    PhysAddr, VirtAddr,
};

use crate::{
    declare_module,
    memory::paging::{
        builder::PageTableBuilder, map::KERNEL_REMAP_PAGE_RANGE, KernelPage, KernelPhysFrame,
        OFFSET_PAGE_TABLE,
    },
    panic::elf::Elf,
    print,
    requests::{EXECUTABLE_ADDRESS, EXECUTABLE_ELF, EXECUTABLE_FILE},
    sprint,
};

pub fn create_kernel_pagetable() -> KernelPhysFrame {
    let mut builder = PageTableBuilder::new(KERNEL_REMAP_PAGE_RANGE);
    // First, map the kernel to the same address as right now.
    map_kernel(&mut builder);

    builder.build()
}

fn map_kernel<T: Iterator<Item = Page>>(builder: &mut PageTableBuilder<T>) {
    let kernel_addr = EXECUTABLE_ADDRESS
        .get()
        .expect("Executable address not initialized");
    let kernel_elf = EXECUTABLE_ELF
        .get()
        .expect("Executable ELF not initialized");
    let opt = {
        // this is gross and a bad way to do this, but because all of the pagetable mapping functions require the global
        // OffsetPageTable, we have to create a new one here.
        // this SHOULD be fine, though this needs to remain immutable
        // this might be able to be moved into the segment_frames iterator, but this might add a huge performance hit
        let cr3 = Cr3::read();
        let off = *crate::requests::PHYSICAL_MEMORY_OFFSET
            .get()
            .expect("Physical memory offset uninitialized");
        let pgtbl = unsafe { &mut *((cr3.0.start_address().as_u64() + off) as *mut PageTable) };
        unsafe { OffsetPageTable::new(pgtbl, VirtAddr::new(off)) }
    };

    for segment in kernel_elf.segments() {
        debug!("Mapping segment: {:?}", segment);
        map_segment(builder, segment, &opt);
    }

    map_stack(builder, &opt);
}

fn map_segment(
    builder: &mut PageTableBuilder<impl Iterator<Item = KernelPage>>,
    segment: &ProgramHeader,
    opt: &OffsetPageTable,
) {
    let pt_flags = segment_to_pt(segment.p_flags);
    let start_page = KernelPage::containing_address(VirtAddr::new(segment.p_vaddr));
    let end_page = KernelPage::containing_address(VirtAddr::new(segment.p_vaddr + segment.p_memsz));
    let mut page_range = KernelPage::range_inclusive(start_page, end_page);

    let mut segment_frames =
        (start_page..=end_page).map(|p| opt.translate_page(p).expect("Failed to translate page"));

    builder.map_range(&mut page_range, &mut segment_frames, pt_flags);
}

fn map_stack(
    builder: &mut PageTableBuilder<impl Iterator<Item = KernelPage>>,
    opt: &OffsetPageTable,
) {
    let stack_base = KernelPage::containing_address(VirtAddr::new(
        *crate::STACK_BASE.get().expect("Stack base not initialized"),
    ));
    let stack_end = KernelPage::containing_address(VirtAddr::new(
        // x86_64 stacks grow downwards
        *crate::STACK_BASE.get().expect("Stack base not initialized") - (crate::STACK_SIZE - 1),
    ));
    let mut stack_range = KernelPage::range_inclusive(stack_end, stack_base);

    let mut segment_frames =
        (stack_end..=stack_base).map(|p| opt.translate_page(p).expect("Failed to translate page"));

    builder.map_range(
        &mut stack_range,
        &mut segment_frames,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );
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

fn init() -> Result<(), Infallible> {
    let kernel_frame = create_kernel_pagetable();
    info!(
        "Kernel page table created with root frame: {:#x?}",
        kernel_frame
    );

    unsafe {
        Cr3::write(kernel_frame, Cr3Flags::empty());
    }

    sprint!(
        "Kernel paging initialized with root frame: {:#x?}\n",
        kernel_frame
    );
    loop {}
    Ok(())
}

declare_module!("kernel_paging", init);
