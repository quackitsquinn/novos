use core::{fmt::Write, time::Duration};

use kserial::{client, common::Command};
use log::info;
use spin::Once;

use crate::{interrupts::hardware::timer::Timer, serial::raw::SerialPort, util::OnceMutex};

pub struct Serial {
    port: SerialPort,
    /// Does the serial support packet mode? (As in, does the other end have packet mode enabled?)
    /// Packet mode in this sense is something that I created, because using multiple serial ports is a
    /// pain and I got tired of fiddling with it.
    ///
    /// In this mode, the serial will send a command byte, followed by the arguments for that command.
    /// For example, a simple print command would be 0x00, followed by the length of the string, followed by the string.
    /// This is useful for debugging, as it allows for a more structured way of sending data.
    ///
    /// This can be enabled by sending the command 0xFF to the serial port.
    ///
    /// This will only be enabled after interrupts are enabled, as the kernel waits for a few cycles to ensure that the other end is ready.
    packet_support: bool,
}

impl Serial {
    /// Initialize the serial port.
    ///
    /// # Safety
    /// The caller must ensure that the port number is valid, and that the port is not already in use.
    pub unsafe fn new(port: u16) -> Self {
        let mut port = unsafe { SerialPort::new(port) };
        port.init();
        for i in 0..10 {
            // Send 10 FF bytes to make sure that and garbage data is cleared out.
            port.send_raw(0xFF);
        }
        Serial {
            port,
            packet_support: false,
        }
    }

    pub fn enable_packet_support(&mut self) {
        // writeln!(self, "Enabling packet support").unwrap();
        self.packet_support = true;
    }

    pub fn disable_packet_support(&mut self) {
        // writeln!(self, "Disabling packet support").unwrap();
        self.packet_support = false;
    }

    pub unsafe fn send_raw(&mut self, data: u8) {
        self.port.send_raw(data);
    }

    pub unsafe fn send_slice_raw(&mut self, data: &[u8]) {
        for byte in data {
            unsafe { self.send_raw(*byte) };
        }
    }

    pub fn has_packet_support(&self) -> bool {
        self.packet_support
    }

    pub unsafe fn get_inner(&mut self) -> &mut SerialPort {
        &mut self.port
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if self.packet_support {
            panic!("Do not use write_str with packet support enabled");
        } else {
            for byte in s.bytes() {
                unsafe {
                    self.send_raw(byte);
                }
            }
        }
        Ok(())
    }
}
