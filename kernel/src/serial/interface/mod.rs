use core::{fmt::Write, time::Duration};

use kserial::{client::SerialAdapter, common::Command};
use serial::Serial;
use spin::Once;

use crate::{interrupts::hardware::timer::Timer, util::OnceMutex};

use super::raw::SerialPort;

pub mod serial;

static SERIAL_PORT: OnceMutex<Serial> = OnceMutex::new();

impl SerialAdapter for OnceMutex<Serial> {
    fn send(&self, data: u8) -> Result<(), kserial::client::WouldBlockError> {
        unsafe {
            self.get()
                .get_inner()
                .try_send_raw(data)
                .map_err(|_| kserial::client::WouldBlockError)
        }
    }

    fn send_slice(&self, data: &[u8]) -> Result<(), kserial::client::WouldBlockError> {
        let mut s = self.get();
        let serial = unsafe { s.get_inner() };
        for byte in data {
            serial
                .try_send_raw(*byte)
                .map_err(|_| kserial::client::WouldBlockError)?;
        }
        Ok(())
    }

    fn read(&self) -> Result<u8, kserial::client::WouldBlockError> {
        unsafe {
            self.get()
                .get_inner()
                .try_receive()
                .map_err(|_| kserial::client::WouldBlockError)
        }
    }

    fn read_slice(&self, data: &mut [u8]) -> Result<usize, kserial::client::WouldBlockError> {
        let mut s = self.get();
        let serial = unsafe { s.get_inner() };
        let mut i = 0;
        for byte in data.iter_mut() {
            *byte = serial
                .try_receive()
                .map_err(|_| kserial::client::WouldBlockError)?;
            i += 1;
        }
        Ok(i)
    }
}

pub fn init() {
    SERIAL_PORT.init(unsafe { Serial::new(0x3F8) });
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let mut serial = SERIAL_PORT.get();
    if serial.has_packet_support() {
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

pub fn init_packet_support() {
    sprintln!("Checking for packet support...");
    let mut serial = SERIAL_PORT.get();
    serial.check_packet_support();
    let support = serial.has_packet_support();
    drop(serial);
    if support {
        sprintln!("Packet support enabled");
    } else {
        sprintln!("Packet support not enabled");
    }
}
