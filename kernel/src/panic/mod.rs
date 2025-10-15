//! Panic handling and stack tracing.

use core::{convert::Infallible, fmt::Write, panic::PanicInfo};

use cake::Fuse;

use crate::{
    declare_module, hlt_loop,
    memory::{self, allocator},
    print, println,
    serial::{self, interface::SERIAL_PORT_NUM, raw::SerialPort},
    testing,
};

pub mod stacktrace;

/// A basic panic handler that just prints the panic message to the serial port.
pub fn panic_basic(pi: &PanicInfo) {
    // Write the raw panic message to the serial port.
    let mut panic_writer = unsafe { SerialPort::new(SERIAL_PORT_NUM) };
    if !serial::is_initialized() {
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
    if memory::allocator::ALLOCATOR.is_initialized() {
        println!("=== HEAP STATE ===");
        println!("Main heap:");
        // Safety: We are in a panic, so the allocator should be completely halted
        let alloc = unsafe { allocator::ALLOCATOR.force_get().unwrap() };
        alloc.print_state();
        // Drop the allocator so that it isn't locked when we print to the screen
        println!("Sending heap state to serial");

        // alloc.blocks.export_block_binary("heap.raw"); TODO: Update this to use the new allocator
    } else {
        println!("Heap allocator not initialized");
    }

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

static PANICKED: Fuse = Fuse::new();

/// Default method for handling panics.
/// This will defer to [panic_basic] if a double panic occurs (e.g. a panic within `panic_extended_info`)
pub fn panic(pi: &PanicInfo) -> ! {
    if PANICKED.is_blown() {
        //println!("Double panic!");
        panic_basic(pi);
        hlt_loop();
    }
    PANICKED.blow();
    panic_extended_info(pi);
    println!("Done; attempting QEMU exit");
    testing::try_shutdown_qemu(true);
    println!("Failed to exit QEMU; halting");
    hlt_loop();
}

declare_module!("panic", init);

fn init() -> Result<(), Infallible> {
    // Nothing to initialize yet, but keeping this here in case we need it later
    Ok(())
}
