#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]

use kernel::{
    ctx::InterruptContext,
    interrupt_wrapper,
    interrupts::set_custom_handler,
    memory::{
        paging::{
            phys::{self, phys_mem::map_address},
            OFFSET_PAGE_TABLE,
        },
        stack::Stack,
    },
    sprintln,
};
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::{
    structures::paging::{Page, PageTableFlags},
    VirtAddr,
};

extern crate alloc;

#[panic_handler]
fn panic(pi: &core::panic::PanicInfo) -> ! {
    kernel::panic::panic(pi);
}

#[unsafe(no_mangle)]
//#[cfg(not(test))]
pub extern "C" fn _start() -> ! {
    use core::arch::asm;

    use kernel::{memory::paging::phys::FRAME_ALLOCATOR, println};
    use x86_64::structures::paging::FrameAllocator;

    set_custom_handler(95, raw_switch_stackless);

    kernel::init_kernel();

    println!("Hello, world!");
    println!("Attempting context switch");
    unsafe { asm!("int 95") };
    println!("Welcome to NovOS!");
    println!("This is a very long line. This will test that the framebuffer SHOULD NOT, I repeat SHOULD NOT crash. blah blah blah blah blah blah blah");

    kernel::hlt_loop()
}

extern "C" fn switch_stackless(int_ctx: *mut InterruptContext) {
    // Set base page to somewhere in the lower half
    let stack =
        Stack::allocate_kernel_stack(0x2000, Page::containing_address(VirtAddr::new(0x100000000)))
            .expect("Unable to allocate stack");
    let base = stack.get_stack_base();
    let rip = VirtAddr::new((test_stackless as usize) as u64);
    unsafe {
        let ctx = &mut *(int_ctx as *mut InterruptContext);
        ctx.int_frame.stack_pointer = base;
        ctx.int_frame.instruction_pointer = rip;
    }
}

interrupt_wrapper!(switch_stackless, raw_switch_stackless);

extern "C" fn test_stackless() -> ! {
    sprintln!("Holy crap context switching works!");
    loop {}
}
