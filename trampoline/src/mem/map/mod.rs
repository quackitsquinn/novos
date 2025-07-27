use core::{num::Wrapping, panic};

use cake::{info, trace};
use kelp::{
    Elf,
    goblin::{
        elf::reloc::{R_X86_64_64, R_X86_64_PC32, R_X86_64_RELATIVE},
        elf64::{program_header::ProgramHeader, reloc::Rela},
    },
    reloc::ElfRelocation,
};
use kvmm::{KernelPage, KernelPhysFrame, pagetable::PagetableBuilder};
use x86_64::{
    VirtAddr,
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, page::PageRange},
};

use crate::{
    arch::{KERNEL_JUMP_LOAD_POINT, copy_jump_point},
    mem::{
        Kernel, MAPPER, PAGETABLE, kernel::MappedPageIterator, map::cfg::VIRTUAL_MAP_PAGE_RANGE,
    },
    requests::KERNEL_FILE,
};

pub mod cfg;

/// Maps the kernel and returns a struct that describes the kernel's memory layout.
pub fn map_kernel() -> Kernel {
    // Setup the pagetable builder with the virtual map page range. We also parse the kernel ELF file.
    let mut iter = MappedPageIterator::from_page_range(VIRTUAL_MAP_PAGE_RANGE);
    let mut builder = PagetableBuilder::new(&mut iter);
    let kernel = KERNEL_FILE.get().expect("Kernel file not found");
    let elf = Elf::new(kernel).expect("Failed to parse ELF file");

    // Map the kernel segments into the pagetable.
    for segment in elf.segments() {
        map_segment(segment, kernel, &mut builder);
    }

    let base_address = elf.segments().map(|s| s.p_vaddr).min().unwrap_or(0);

    // If the kernel has a relocation section, we apply the relocations.
    // for rela in elf.relocations().expect("Failed to get relocations") {
    //     apply_relocation(&rela, &elf, base_address, &mut builder);
    // }

    // Copy the trampoline code to the jump point page.
    // This is crucial for the kernel to be able to jump to its entry point.
    let (jump_page, jump_frame) = builder.next_page();
    unsafe {
        copy_jump_point(jump_page);
    }
    builder.map_page(
        KERNEL_JUMP_LOAD_POINT,
        jump_frame,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    // Map the stack and return the stack pointer.
    let stack_ptr = map_stack(&mut builder);

    // Build the pagetable. We don't need to worry about releasing the resources since the resources will be disposed
    // as soon as the kernel is loaded.
    let (pgtbl, frame, _) = builder.build_and_release(None);

    // Find the entry point of the kernel.
    // This is done by looking for the `_start` symbol in the ELF file.
    let rip = find_entrypoint(&elf);

    // We don't want drop code to run on the pagetable, so we convert it into a raw pointer.
    let _ = unsafe { pgtbl.into_raw() };

    unsafe {
        Kernel::new(
            frame,
            rip.expect("Kernel entry point not found!"),
            stack_ptr,
        )
    }
}

fn map_stack<T>(builder: &mut PagetableBuilder<T>) -> VirtAddr
where
    T: Iterator<Item = (KernelPage, KernelPhysFrame)>,
{
    let stack_page_range = KernelPage::range_inclusive(
        KernelPage::containing_address(cfg::STACK_TOP),
        KernelPage::containing_address(VirtAddr::new_truncate(
            cfg::STACK_TOP.as_u64() + cfg::STACK_SIZE as u64 - 1,
        )),
    );

    for page in stack_page_range {
        let (copy_page, dest_frame) = builder.next_page();
        unsafe {
            copy_page
                .start_address()
                .as_mut_ptr::<u8>()
                .write_bytes(0, KernelPage::SIZE as usize);
        }
        builder.map_page(
            page,
            dest_frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );
    }
    VirtAddr::new_truncate(cfg::STACK_TOP.as_u64() + cfg::STACK_SIZE as u64 - 8)
}

fn map_segment<T>(segment: &ProgramHeader, kernel: &[u8], builder: &mut PagetableBuilder<T>)
where
    T: Iterator<Item = (KernelPage, KernelPhysFrame)>,
{
    // The alignment is too high and we probably can't map it correctly.
    if segment.p_align > KernelPage::SIZE {
        panic!("Segment alignment is too large: {}", segment.p_align);
    }

    // If the segment has no memory size, we skip it.
    if segment.p_memsz == 0 {
        return;
    }

    // Convert the segment flags to page table flags.
    let flags = segment_to_pt(segment.p_flags);

    let virt_addr = VirtAddr::new(segment.p_vaddr);
    let mut virt_range = Page::range_inclusive(
        KernelPage::containing_address(virt_addr),
        KernelPage::containing_address(virt_addr + segment.p_memsz as u64 - 1),
    );

    let offset = segment.p_offset as usize;
    let kernel_pages = kernel[offset..offset + segment.p_filesz as usize].chunks(4096);

    // In this loop we copy the data from the kernel into it's own frame, that is then mapped to the virtual address in the builder.
    // In the future this might be changed to just translate the original kernel binary into it's physical frame, but for now we just copy it.
    for data in kernel_pages {
        let dest_page = virt_range.next().expect("virt range exhausted");
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

fn find_entrypoint(elf: &Elf) -> Option<VirtAddr> {
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

        if name == cfg::ENTRY_PONT_NAME {
            return Some(VirtAddr::new(sym.st_value));
        }
    }
    None
}

// fn apply_relocation(
//     rela: &ElfRelocation,
//     elf: &Elf,
//     base_addr: u64,
//     builder: &mut PagetableBuilder<&mut MappedPageIterator>,
// ) {
//     let temp_page = builder.iterator().next_page();
//     let write_reloc = |addr: u64, value: u64| {
//         let page = KernelPage::containing_address(VirtAddr::new(addr));
//         let page_offset = addr - page.start_address().as_u64();
//         let frame = builder
//             .frame(page)
//             .expect("Failed to get frame for relocation");
//         let mut mapper = MAPPER.get().expect("uninit").lock();
//         let mut table = PAGETABLE.get().expect("uninit").lock();
//         unsafe {
//             table.map_to(
//                 temp_page,
//                 frame,
//                 PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE | PageTableFlags::WRITABLE,
//                 &mut *mapper,
//             )
//         }
//         .expect("Failed to map page")
//         .flush();
//         let ptr = unsafe {
//             temp_page
//                 .start_address()
//                 .as_mut_ptr::<u64>()
//                 .add(page_offset as usize)
//         };
//         unsafe {
//             ptr.write_unaligned(value);
//         }
//         table
//             .unmap(temp_page)
//             .expect("Failed to unmap page")
//             .1
//             .flush();
//     };

//     let dest = (rela.offset.wrapping_add(base_addr)) as u64;

//     let S = if rela.info.index() != 0 {
//         let sym = elf.symbols_slice().expect("Failed to get symbols")[rela.info.index() as usize];
//         sym.st_value
//     } else {
//         0
//     };

//     let base_addr = Wrapping(base_addr);
//     let S = Wrapping(S);
//     let A = Wrapping(rela.addend as u64);
//     let P = Wrapping(base_addr.0.wrapping_add(rela.offset as u64));

//     let val = match rela.info.reloc_type() as u32 {
//         R_X86_64_RELATIVE => base_addr + A,
//         R_X86_64_64 => S + A,
//         R_X86_64_PC32 => S + A - P,
//         _ => panic!("Unsupported relocation type: {:?}", rela.info.reloc_type()),
//     };
//     write_reloc(dest, val.0);
// }

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
