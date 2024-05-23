#![no_std]
#![no_main]

use kernel::{display::color::Color, sprintln};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sprintln!("uh oh, the code {}", _info);
    kernel::hlt_loop();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    sprintln!("Hello World!");
    let mut buf = kernel::display::FRAMEBUFFER.lock();
    for x in 0..buf.width() {
        for y in 0..buf.height() {
            buf.set_px(x, y, Color::new(128, 128, 128));
        }
    }

    kernel::hlt_loop();
}
