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
    kernel::display::FRAMEBUFFER.lock().draw_scaled_sprite(
        0,
        0,
        4,
        &kernel::display::get_char(unsafe { char::from_u32_unchecked(0xfff) }), // test invalid char
        Color::new(0, 255, 0),
    );
    kernel::hlt_loop();
}
