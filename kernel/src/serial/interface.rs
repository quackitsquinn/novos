use crate::util::OnceMutex;

use super::raw::SerialPort;

const SERIAL_PORT_NUM: u16 = 0x3F8;
static SERIAL_PORT: OnceMutex<SerialPort> = OnceMutex::new();

pub fn init() {
    SERIAL_PORT.init({
        let mut port = unsafe { SerialPort::new(0x3F8) };
        port.init();
        port
    });
}

pub fn write_byte(byte: u8) {
    SERIAL_PORT.get().send(byte);
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

static mut MISSED_MSG: bool = false;

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    if SERIAL_PORT.is_locked() {
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
    SERIAL_PORT.get().write_fmt(args).unwrap();
}
/// Serial print
#[macro_export]
macro_rules! sprint {
    ($($arg:tt)*) => {
        $crate::serial::interface::_print(format_args!($($arg)*))
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
