#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use kernel::memory::paging::{
    phys::{self, phys_mem::map_address},
    OFFSET_PAGE_TABLE,
};
use x86_64::structures::paging::PageTableFlags;

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

    kernel::init_kernel();
    #[no_mangle]
    fn i0() {
        #[no_mangle]
        fn i1() {
            #[no_mangle]
            fn i2() {
                #[no_mangle]
                fn i3() {
                    unsafe {
                        asm!("int 14");
                    }
                    panic!("Page fault not thrown");
                }
                i3()
            }
            i2()
        }
        i1()
    }
    i0();

    println!("Hello, world!");
    println!("Welcome to NovOS!");
    println!("This is a very long line. This will test that the framebuffer SHOULD NOT, I repeat SHOULD NOT crash. blah blah blah blah blah blah blah");

    kernel::hlt_loop()
}
