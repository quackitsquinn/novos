use core::{fmt::Write, time::Duration};

use kserial::{client::SerialAdapter, common::Command};
use serial::Serial;
use spin::Once;

use crate::{interrupts::hardware::timer::Timer, util::OnceMutex};

use super::raw::SerialPort;

pub mod serial;

pub static SERIAL_PORT: OnceMutex<Serial> = OnceMutex::new();
// TODO: Abstract this and similar things into a Lock type that just has the like is_locked etc.
pub static PORT_HAS_INIT: Once<()> = Once::new();

impl SerialAdapter for OnceMutex<Serial> {
    // HACK: All the .force_unlock() calls are to unlock the port, which is weird. I don't know why it's locked in the first place, but this fixed the previous issue.
    fn send(&self, data: u8) {
        unsafe {
            self.force_unlock();
            let mut s = self.get();

            s.get_inner().send_raw(data);
        }
    }

    fn send_slice(&self, data: &[u8]) {
        unsafe {
            self.force_unlock();
        }
        let mut s = self.get();
        let serial = unsafe { s.get_inner() };
        for byte in data {
            serial.send_raw(*byte);
        }
    }

    fn read(&self) -> u8 {
        unsafe {
            self.force_unlock();
        }
        let mut s = self.get();
        unsafe { s.get_inner().receive() }
    }

    fn read_slice(&self, data: &mut [u8]) -> usize {
        unsafe {
            self.force_unlock();
        }
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
    let mut serial = SERIAL_PORT.get();
    kserial::client::init(&SERIAL_PORT);
    serial.enable_packet_support();
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let mut serial = SERIAL_PORT.get();
    if serial.has_packet_support() {
        //writeln!(serial, "AUIFNHAWERIOUGHAWEJIOGJAEHLUIGNAWOI").unwrap();
        Command::WriteArguments(&args).send();
    } else {
        serial.write_fmt(args).unwrap();
    }
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
