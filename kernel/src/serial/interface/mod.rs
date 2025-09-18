use core::fmt::Write;

use kserial::client::SerialAdapter;
use serial::Serial;
use spin::Once;

pub mod serial;

// TODO: Abstract this and similar things into a Lock type that just has the like is_locked etc.
pub static PORT_HAS_INIT: Once<()> = Once::new();

pub fn init() {
    static mut SERIAL_PORT: Option<Serial> = None;
    unsafe {
        if SERIAL_PORT.is_none() {
            SERIAL_PORT = Some(Serial::new(0x3F8));
        }
    }
    PORT_HAS_INIT.call_once(|| ());
    kserial::client::init(unsafe {
        &mut *SERIAL_PORT.as_mut().expect("Serial port not initialized")
    });
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    write!(kserial::client::writer(), "{}", args).unwrap();
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
