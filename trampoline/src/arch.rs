use core::arch::naked_asm;

use cake::debug;
use kvmm::KernelPage;
use x86_64::{
    VirtAddr,
    structures::paging::{Mapper, PageTableFlags},
};

use crate::mem::{Kernel, MAPPER, PAGETABLE};

pub const KERNEL_JUMP_LOAD_POINT: KernelPage =
    KernelPage::containing_address(VirtAddr::new_truncate(0x1_000_000_000));

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".trampoline")]
unsafe extern "sysv64" fn jump_inner(cr3: u64, rip: u64, rsp: u64) -> ! {
    naked_asm! {
        // Load the CR3 register with the kernel's page table base address
        "mov cr3, rdi",
        // Set the stack pointer to the kernel's stack
        "mov rsp, rdx",
        // Set the instruction pointer to the kernel's entry point
        "jmp rsi",
        // Disable interrupts
        "cli",
        // Halt the CPU indefinitely. If we made it here, something has gone, terribly, terribly, wrong.
        "hlt",

    };
}

unsafe extern "C" {
    static trampoline_start: u8;
    static trampoline_end: u8;
}

/// Copies the trampoline code to the specified page.
///
/// # Safety
/// The caller must ensure that the destination page is valid and writable.
pub unsafe fn copy_jump_point(dest: KernelPage) {
    let start = unsafe { &trampoline_start as *const u8 as usize };
    let end = unsafe { &trampoline_end as *const u8 as usize };
    let size = end - start;
    debug!(
        "Copying trampoline code from {:#x} to {:#x} (size: {})",
        start,
        dest.start_address().as_u64(),
        size
    );
    let ptr = start as *const u8;
    let page_ptr = dest.start_address().as_mut_ptr::<u8>();
    unsafe {
        page_ptr.copy_from_nonoverlapping(ptr, size);
    }
}

/// Loads the jump point into the page table.
pub fn load_jump_point() {
    let mut table = PAGETABLE.get().expect("uninit").lock();
    let mut mapper = MAPPER.get().expect("uninit").lock();
    let frame = mapper.next_frame().expect("no frame available");
    let page = KERNEL_JUMP_LOAD_POINT;
    unsafe {
        table
            .map_to(
                page,
                frame,
                PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                &mut *mapper,
            )
            .expect("Failed to map page")
            .flush();
    }

    unsafe { copy_jump_point(page) };
}

/// Jumps to the kernel's entry point.
/// # Safety
/// The caller must ensure that the kernel's CR3, RIP, and RSP are valid.
/// The caller must also ensure that the trampoline code has been loaded at `KERNEL_JUMP_LOAD_POINT` in both the active pagetable and in the given cr3.
pub unsafe fn jump(kernel: Kernel) -> ! {
    let Kernel { cr3, rip, rsp } = kernel;

    let ptr = KERNEL_JUMP_LOAD_POINT
        .start_address()
        .as_ptr::<unsafe extern "sysv64" fn(u64, u64, u64) -> !>();

    unsafe { (*ptr)(cr3.start_address().as_u64(), rip.as_u64(), rsp.as_u64()) }
}
