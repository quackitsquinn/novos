use core::convert::Infallible;

use log::{debug, info};
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        page::PageRange, page_table::PageTableEntry, Page, PageTable, PageTableFlags,
        PageTableIndex, PhysFrame, RecursivePageTable,
    },
    PhysAddr, VirtAddr,
};

use crate::{
    declare_module,
    memory::paging::{
        builder::PageTableBuilder, map::KERNEL_REMAP_PAGE_RANGE, KernelPage, KernelPhysFrame,
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

    for segment in kernel_elf.segments() {
        info!("Mapping section: {:?}", segment);
    }
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
