//! Serial port driver for debug output.
//!
//! This module is based off of the uart_16550 crate, which is a driver for the 16550 UART chip.
//! I would use that, but I 1. wanted to write my own, and 2. couldn't figure out how to get cargo
//! to stop reporting the correct struct as configured out.

use crate::{Mutex, Once, OnceMut};
use core::arch::asm;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts::without_interrupts;

const SERIAL_PORT_NUM: u16 = 0x3F8;

static PORT: OnceMut<SerialPort> = OnceMut::new();

pub fn init() {
    PORT.call_once(|| {
        let mut port = unsafe { SerialPort::new(SERIAL_PORT_NUM) };
        port.init();
        Mutex::new(port)
    });
}

pub fn write_byte(byte: u8) {
    PORT.get().unwrap().lock().send(byte);
}

pub fn write_str(s: &str) {
    without_interrupts(|| {
        for byte in s.bytes() {
            write_byte(byte);
        }
    });
}

static mut MISSED_MSG: bool = false;

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    if PORT.get().unwrap().is_locked() {
        // If the port is locked, we can't write to it, so just return.
        // TODO: Use a Vec as a buffer when the allocator is implemented.
        unsafe {
            MISSED_MSG = true;
        }
        return;
    } else if unsafe { MISSED_MSG } {
        // If we missed a message, print a message saying so.
        write_str("Missed message\n");
        unsafe {
            MISSED_MSG = false;
        }
    }
    PORT.get().unwrap().lock().write_fmt(args).unwrap();
}
/// Serial print
#[macro_export]
macro_rules! sprint {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}
/// Serial print with newline
#[macro_export]
macro_rules! sprintln {
    () => {
        $crate::sprint!("\n");
    };
    ($fmt:expr) => {
        $crate::sprint!(concat!($fmt, "\n"));
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::sprint!(concat!($fmt, "\n"), $($arg)*);
    };

}
