//! Sets up the kernel serial interface.
use core::fmt::Write;

use cake::spin::Once;
use cake::spin::{Mutex, MutexGuard};
use kserial::client::SerialAdapter;
use serial::Serial;

pub mod serial;

/// The I/O port number for the primary serial port (COM1).
pub const SERIAL_PORT_NUM: u16 = 0x3F8; // COM1

static SERIAL_PORT: Once<Mutex<Serial>> = Once::new();

pub(super) fn init() {
    SERIAL_PORT.call_once(|| Mutex::new(unsafe { Serial::new(SERIAL_PORT_NUM) }));
    kserial::client::init(
        MutexGuard::leak(unsafe { SERIAL_PORT.get_unchecked().lock() }) as &mut dyn SerialAdapter,
    );
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let mut writer = kserial::client::writer();
    write!(writer, "{}", args).unwrap();
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
