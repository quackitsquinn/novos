use core::{fmt::Write, time::Duration};

use kserial::{client, common::Command};
use log::info;

use crate::{interrupts::hardware::timer::Timer, serial::raw::SerialPort, util::OnceMutex};

const SERIAL_PORT_NUM: u16 = 0x3F8;
static SERIAL_PORT: OnceMutex<Serial> = OnceMutex::new();
const PACKET_SUPPORT_WAIT_TIME: Duration = Duration::from_millis(100);

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
        Serial {
            port,
            packet_support: false,
        }
    }

    pub fn check_packet_support(&mut self) {
        let timer = Timer::new(PACKET_SUPPORT_WAIT_TIME);
        let mut has_read = false;
        // Write a 0xFF byte to the other end to indicate that we have started polling for packet support.
        unsafe {
            self.send_raw(0xFF);
        }

        while !timer.is_done() && !has_read {
            writeln!(self, "Checking for packet support...").unwrap();
            if let Ok(byte) = self.port.try_receive() {
                // Similar to TCP/IP's SYN-ACK handshake, we send a 0xFF byte to the other end to indicate that we are ready for packet mode.
                if byte == 0xFF {
                    unsafe {
                        self.send_raw(0xFF);
                    }
                    self.packet_support = true;
                    client::init(&SERIAL_PORT);
                    has_read = true;
                }
            }
        }
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
            panic!("Do not call this function! Use the Command enum instead.");
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
