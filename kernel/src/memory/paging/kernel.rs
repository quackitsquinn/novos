//! Big ol' file that handles setting up the kernel's page tables.
//! In the future, this will almost certainly be converted into it's own module.
use core::convert::Infallible;

use cake::{spin::Once, terminate_requests};
use kelp::goblin::elf64::program_header::ProgramHeader;
use cake::log::{debug, info, trace};
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        frame::PhysFrameRangeInclusive,
        mapper::{MappedFrame, TranslateResult},
        OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, RecursivePageTable, Translate,
    },
    VirtAddr,
};

use crate::{
    declare_module,
    memory::paging::{
        builder::PageTableBuilder,
        map::{FRAMEBUFFER_START_PAGE, KERNEL_REMAP_PAGE_RANGE},
        KernelPage, KernelPhysFrame, KERNEL_PAGE_TABLE,
    },
    mp::mp_setup::CORES,
    requests::{FRAMEBUFFER, KERNEL_ELF},
};

pub fn create_kernel_pagetable() -> (KernelPhysFrame, KernelPage) {
    let mut builder = PageTableBuilder::new(KERNEL_REMAP_PAGE_RANGE);
    // First, map the kernel to the same address as right now.
    map_kernel(&mut builder);

    builder.build()
}

fn map_kernel<T: Iterator<Item = Page>>(builder: &mut PageTableBuilder<T>) {
    let kernel_elf = KERNEL_ELF.get().elf();
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

    map_stack(
        builder,
        &opt,
        *crate::STACK_BASE.get().expect("Stack base uninitialized"),
    );

    for (i, base) in CORES.get().expect("cores uninit").iter() {
        info!("Mapping stack for CPU with APIC ID {}...", i);

        map_stack(builder, &opt, *base.read().get_stack_start().wait());
    }

    map_framebuffer(builder, &opt);
    map_heap(builder, &opt);
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

    builder.map_range(
        &mut page_range,
        &mut CopyPages::new(start_page..=end_page, opt),
        pt_flags,
    );
}

fn map_stack(
    builder: &mut PageTableBuilder<impl Iterator<Item = KernelPage>>,
    opt: &OffsetPageTable,
    base: u64,
) {
    let stack_base = KernelPage::containing_address(VirtAddr::new(base));
    let stack_end = KernelPage::containing_address(VirtAddr::new(
        // x86_64 stacks grow downwards
        base - (crate::STACK_SIZE - 1),
    ));
    let mut stack_range = KernelPage::range_inclusive(stack_end, stack_base);

    builder.map_range(
        &mut stack_range,
        &mut CopyPages::new(stack_end..=stack_base, opt),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );
}

fn map_framebuffer(
    builder: &mut PageTableBuilder<impl Iterator<Item = KernelPage>>,
    opt: &OffsetPageTable,
) {
    let fb = FRAMEBUFFER.get();

    let size = (fb.height * fb.pitch) as u64;
    let root = VirtAddr::from_ptr(unsafe { fb.ptr_unchecked() });

    info!(
        "Mapping framebuffer at {:#x} of size {:#x}",
        root.as_u64(),
        size
    );

    let start_page = KernelPage::containing_address(root);
    let end_page = KernelPage::containing_address(VirtAddr::new(root.as_u64() + size - 1));
    let old_page_range = start_page..=end_page;

    let start_page = FRAMEBUFFER_START_PAGE;
    let end_page = KernelPage::containing_address(VirtAddr::new(
        FRAMEBUFFER_START_PAGE.start_address().as_u64() + size - 1,
    ));
    let mut new_page_range = start_page..=end_page;

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;

    builder.map_range(
        &mut new_page_range,
        &mut CopyPages::new(old_page_range, opt),
        flags,
    );

    info!(
        "Framebuffer mapped at {:#x}\n",
        start_page.start_address().as_u64()
    );

    unsafe {
        fb.update_ptr(remap_ptr(fb.ptr_unchecked(), start_page).cast_mut());
    }
}

fn map_heap(
    builder: &mut PageTableBuilder<impl Iterator<Item = KernelPage>>,
    opt: &OffsetPageTable,
) {
    let heap_start = crate::memory::paging::map::KERNEL_HEAP_START_PAGE;
    let heap_end = crate::memory::paging::map::KERNEL_HEAP_END_PAGE;
    let mut heap_range = KernelPage::range(heap_start, heap_end);

    builder.map_range(
        &mut heap_range,
        &mut CopyPages::new(heap_start..heap_end, opt),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );
}

/// A helper struct to copy pages from one page table to another.
/// This struct implements `Iterator` and yields the physical frames corresponding to the pages
/// in the inner iterator. Hence why it needs a reference to the `OffsetPageTable` to perform the translation.
struct CopyPages<'a, T: Iterator<Item = KernelPage>> {
    iter: T,
    pt: &'a OffsetPageTable<'a>,
    curr: Option<PhysFrameRangeInclusive>,
}

const FRAMES_PER_2MIB: u64 = 512;
const FRAMES_PER_1GIB: u64 = 512 * 512;

impl<'a, T: Iterator<Item = KernelPage>> CopyPages<'a, T> {
    fn new(iter: T, pt: &'a OffsetPageTable) -> Self {
        Self {
            iter,
            pt,
            curr: None,
        }
    }

    fn next_cached_page(&mut self) -> Option<KernelPhysFrame> {
        if let Some(ref mut range) = self.curr {
            if let Some(frame) = range.next() {
                return Some(frame);
            } else {
                self.curr = None;
                return None;
            }
        }
        None
    }

    /// Expands a large frame (2MiB or 1GiB) into 4KiB frames.
    /// This updates the internal state of the iterator to yield the remaining frames
    /// in subsequent calls to `next()`.
    fn explode_frame(&mut self, frame: MappedFrame) -> KernelPhysFrame {
        match frame {
            MappedFrame::Size4KiB(f) => {
                // nothing to do here, just return the frame
                return f;
            }
            MappedFrame::Size2MiB(f) => {
                trace!("Expanding 2MiB frame {:#x?}", f);
                let f = KernelPhysFrame::from_start_address(f.start_address()).unwrap();
                self.curr = Some(PhysFrame::range_inclusive(f + 1, f + FRAMES_PER_2MIB));
                self.iter
                    .nth((FRAMES_PER_2MIB - 1) as usize)
                    .expect("Iterator ended early");
                f
            }

            MappedFrame::Size1GiB(f) => {
                trace!("Expanding 1GiB frame {:#x?}", f);
                let f = KernelPhysFrame::from_start_address(f.start_address()).unwrap();
                self.curr = Some(PhysFrame::range_inclusive(f + 1, f + FRAMES_PER_1GIB - 1));
                self.iter
                    .nth((FRAMES_PER_1GIB - 1) as usize)
                    .expect("Iterator ended early");
                f
            }
        }
    }
}

impl<'a, T: Iterator<Item = KernelPage>> Iterator for CopyPages<'a, T> {
    type Item = KernelPhysFrame;

    fn next(&mut self) -> Option<Self::Item> {
        // first, check if we have a current range to yield from
        if let Some(range) = self.next_cached_page() {
            return Some(range);
        }

        // otherwise, get the next page from the iterator and translate it.
        let next_page = self.iter.next()?;
        let next = self.pt.translate(next_page.start_address());

        let frame = match next {
            TranslateResult::Mapped { frame, .. } => frame,
            TranslateResult::NotMapped => panic!("Page not mapped in source page table"),
            TranslateResult::InvalidFrameAddress(a) => {
                panic!("Invalid frame address: {:?}", a)
            }
        };

        Some(self.explode_frame(frame))
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

/// Remap a pointer from its current location to a new page.
/// The offset within the page is preserved.
fn remap_ptr(ptr: *const u8, new_page: KernelPage) -> *const u8 {
    let offset = ptr as u64 & 0xfff;
    (new_page.start_address().as_u64() + offset) as *const u8
}

pub static KERNEL_CR3: Once<KernelPhysFrame> = Once::new();

fn init() -> Result<(), Infallible> {
    let (kernel_frame, new_ptr) = create_kernel_pagetable();
    info!(
        "Kernel page table created with root frame: {:#x?}",
        kernel_frame
    );
    KERNEL_CR3.call_once(|| kernel_frame);

    unsafe {
        Cr3::write(kernel_frame, Cr3Flags::empty());
    }

    info!(
        "Kernel paging initialized with root frame: {:#x?}! Switching KernelPageTable.\n",
        kernel_frame
    );

    KERNEL_PAGE_TABLE.write().switch(
        RecursivePageTable::new(unsafe { &mut *new_ptr.start_address().as_mut_ptr::<PageTable>() })
            .unwrap(),
    );

    unsafe { terminate_requests() };

    Ok(())
}

declare_module!("kernel_paging", init);
