#![no_std]
#![no_main]

#[panic_handler]
fn panic(pi: &core::panic::PanicInfo) -> ! {
    kernel::panic::panic_extended_info(pi);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();

    recurse_panic(30);

    kernel::hlt_loop()
}

#[inline(never)]
fn recurse_panic(count: u64) {
    if count == 0 {
        panic!("Recursion limit reached");
    }
    recurse_panic(count - 1);
}
