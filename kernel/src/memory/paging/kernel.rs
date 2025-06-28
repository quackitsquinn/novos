use core::convert::Infallible;

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

    for (indexes, pagetable) in builder.pagetables.iter() {
        info!("{:?}", indexes);
        for page in pagetable
            .1
            .iter()
            .filter(|p| p.flags().contains(PageTableFlags::PRESENT))
        {
            info!("{:?}", page);
        }
    }

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
        info!("Mapping section: {:?}", segment);
        let pt_flags = segment_to_pt(segment.p_flags);
        let start_page = KernelPage::containing_address(VirtAddr::new(segment.p_vaddr));
        let end_page =
            KernelPage::containing_address(VirtAddr::new(segment.p_vaddr + segment.p_memsz));
        let mut page_range = KernelPage::range_inclusive(start_page, end_page);
        let mut segment_frames = kernel_elf.data
            [segment.p_offset as usize..=(segment.p_offset + segment.p_filesz) as usize]
            .chunks(4096)
            .map(|c| {
                opt.translate_page(KernelPage::containing_address(VirtAddr::new(
                    c.as_ptr() as usize as u64,
                )))
                .expect("Failed to translate page")
            });

        builder.map_range(&mut page_range, &mut segment_frames, pt_flags);
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
