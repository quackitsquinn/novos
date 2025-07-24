use core::fmt::Write;

use cake::Once;
use serial::Serial;

pub mod serial;

pub static SERIAL_PORT: Once<Serial> = Once::new();
// TODO: Abstract this and similar things into a Lock type that just has the like is_locked etc.
pub static PORT_HAS_INIT: Once<()> = Once::new();

pub fn init() {
    let port = SERIAL_PORT.call_once(|| unsafe { Serial::new(0x3F8) });
    PORT_HAS_INIT.call_once(|| ());
    kserial::client::init(port);
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    write!(kserial::client::SerialWriter, "{}", args).unwrap();
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
