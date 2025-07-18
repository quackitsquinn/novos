use core::{convert::Infallible, fmt::Write, panic::PanicInfo};

use cake::{declare_module, Once};

use crate::{
    hlt_loop,
    //    memory::{self, allocator},
    print,
    println,
    serial::{self, raw::SerialPort},
    testing,
};

pub mod stacktrace;

pub fn panic_basic(pi: &PanicInfo) {
    // Write the raw panic message to the serial port. This is just a debug tool because somewhere in my serial implementation its just.. exploding.
    let mut panic_writer = unsafe { SerialPort::new(serial::SERIAL_PORT_NUM) };
    if !serial::interface::PORT_HAS_INIT.is_completed() {
        // If the code crashed before the serial port was initialized, we need to initialize it now.
        panic_writer.init();
    }
    let _ = panic_writer.write_str("Panic: ");
    let _ = if let Some(location) = pi.location() {
        panic_writer.write_fmt(format_args!("{}:{}", location.file(), location.line()))
    } else {
        panic_writer.write_str("Unknown location")
    };
    let _ = panic_writer.write_str("\n");
    let _ = panic_writer.write_fmt(format_args!("{}", pi.message()));
    let _ = panic_writer.write_str("\n");
}

/// A more traditional panic handler that includes more information.
pub fn panic_extended_info(pi: &PanicInfo) {
    println!("=== KERNEL PANIC ===");
    print!("Panic at ");
    write_location(pi);
    println!();
    println!("{}", pi.message());
    // if memory::allocator::ALLOCATOR.is_initialized() {
    //     println!("=== HEAP STATE ===");
    //     println!("Main heap:");
    //     // Safety: We are in a panic, so the allocator should be completely halted
    //     let alloc = unsafe { allocator::ALLOCATOR.force_get().unwrap() };
    //     alloc.print_state();
    //     // Drop the allocator so that it isn't locked when we print to the screen
    //     println!("Sending heap state to serial");

    //     // alloc.blocks.export_block_binary("heap.raw"); TODO: Update this to use the new allocator
    // } else {
    //     println!("Heap allocator not initialized");
    // }

    println!("=== STACK TRACE ===");
    stacktrace::print_trace();
    println!("=== END OF PANIC ===");
}

fn write_location(pi: &PanicInfo) {
    if let Some(location) = pi.location() {
        print!("{}:{}", location.file(), location.line())
    } else {
        print!("Unknown location")
    }
}

static PANIC_CHECK: Once<()> = Once::new();

pub fn panic(pi: &PanicInfo) -> ! {
    if PANIC_CHECK.is_completed() {
        //println!("Double panic!");
        panic_basic(pi);
        hlt_loop();
    }
    PANIC_CHECK.call_once(|| ());
    panic_extended_info(pi);
    println!("Done; attempting QEMU exit");
    testing::try_shutdown_qemu(true);
    println!("Failed to exit QEMU; halting");
    hlt_loop();
}

declare_module!("panic", init);

fn init() -> Result<(), Infallible> {
    stacktrace::init();
    Ok(())
}
