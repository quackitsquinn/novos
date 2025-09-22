use core::fmt::Write;

use cake::spin::{Mutex, MutexGuard};
use kserial::client::SerialAdapter;
use serial::Serial;
use spin::Once;

pub mod serial;

// TODO: Abstract this and similar things into a Lock type that just has the like is_locked etc.
pub static PORT_HAS_INIT: Once<()> = Once::new();

pub static SERIAL_PORT_NUM: u16 = 0x3F8; // COM1

pub fn init() {
    static SERIAL_PORT: Mutex<Option<Serial>> = Mutex::new(None);
    SERIAL_PORT
        .lock()
        .replace(unsafe { Serial::new(SERIAL_PORT_NUM) });
    PORT_HAS_INIT.call_once(|| ());
    kserial::client::init(
        MutexGuard::leak(SERIAL_PORT.lock())
            .as_mut()
            .expect("infaliable") as &mut dyn SerialAdapter,
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
