use core::fmt::Write;

use kserial::client::SerialAdapter;
use serial::Serial;
use spin::Once;

use crate::util::OnceMutex;

pub mod serial;

pub static SERIAL_PORT: OnceMutex<Serial> = OnceMutex::uninitialized();
// TODO: Abstract this and similar things into a Lock type that just has the like is_locked etc.
pub static PORT_HAS_INIT: Once<()> = Once::new();

impl SerialAdapter for OnceMutex<Serial> {
    fn send(&self, data: u8) {
        let mut s = self.get();

        unsafe { s.get_inner().send_raw(data) };
    }

    fn send_slice(&self, data: &[u8]) {
        let mut s = self.get();
        let serial = unsafe { s.get_inner() };
        for byte in data {
            serial.send_raw(*byte);
        }
    }

    fn read(&self) -> u8 {
        let mut s = self.get();
        unsafe { s.get_inner().receive() }
    }

    fn read_slice(&self, data: &mut [u8]) -> usize {
        let mut s = self.get();
        let serial = unsafe { s.get_inner() };
        let mut i = 0;
        for byte in data.iter_mut() {
            // TODO: Implement a timeout
            *byte = serial.receive();
            i += 1;
        }
        i
    }
}

pub fn init() {
    SERIAL_PORT.init(unsafe { Serial::new(0x3F8) });
    PORT_HAS_INIT.call_once(|| ());
    let serial = SERIAL_PORT.get();
    kserial::client::init(&SERIAL_PORT);
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
