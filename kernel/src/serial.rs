//! Serial port driver for debug output.
//!
//! This module is based off of the uart_16550 crate, which is a driver for the 16550 UART chip.
//! I would use that, but I 1. wanted to write my own, and 2. couldn't figure out how to get cargo
//! to stop reporting the correct struct as configured out.

use crate::{util::OnceMutex, Mutex, Once};
use core::arch::asm;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts::without_interrupts;

const SERIAL_PORT_NUM: u16 = 0x3F8;

static PORT: OnceMutex<SerialPort> = OnceMutex::new();

pub fn init() {
    PORT.init({
        let mut port = unsafe { SerialPort::new(SERIAL_PORT_NUM) };
        port.init();
        port
    });

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LOG_LEVEL.to_level_filter()))
        .unwrap();
}

pub fn write_byte(byte: u8) {
    PORT.get().send(byte);
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
    if PORT.is_locked() {
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
    PORT.get().write_fmt(args).unwrap();
}
/// Serial print
#[macro_export]
macro_rules! sprint {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
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
        $crate::sprint!(concat!($fmt, "\n"), $($arg)*)
    };

}

const LOG_LEVEL: log::Level = log::Level::Trace;
struct SerialLog;

impl log::Log for SerialLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            sprintln!(
                "[{}] ({}:{}) {}: {}",
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

const LOGGER: SerialLog = SerialLog;
