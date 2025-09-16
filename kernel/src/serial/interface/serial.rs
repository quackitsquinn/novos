use core::fmt::Write;

use kserial::client::{get_serial_client, send_string, SerialAdapter};

use crate::serial::raw::SerialPort;

pub struct Serial {
    port: SerialPort,
}

impl Serial {
    /// Initialize the serial port.
    ///
    /// # Safety
    /// The caller must ensure that the port number is valid, and that the port is not already in use.
    pub unsafe fn new(port: u16) -> Self {
        let mut port = unsafe { SerialPort::new(port) };
        port.init();

        Serial { port }
    }

    pub fn enable_packet_support(&mut self) {
        // writeln!(self, "Enabling packet support").unwrap();
        get_serial_client().enable_packet_support();
    }

    pub fn disable_packet_support(&mut self) {
        // writeln!(self, "Disabling packet support").unwrap();
        todo!("disable_packet_support is not implemented yet");
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
        true
    }

    pub unsafe fn get_inner(&mut self) -> &mut SerialPort {
        &mut self.port
    }
}

impl SerialAdapter for Serial {
    fn send(&mut self, data: u8) {
        unsafe {
            self.send_raw(data);
        };
    }

    fn send_slice(&mut self, data: &[u8]) {
        let serial = unsafe { self.get_inner() };
        for byte in data {
            serial.send_raw(*byte);
        }
    }

    fn read(&mut self) -> u8 {
        unsafe { self.get_inner().receive() }
    }

    fn read_slice(&mut self, data: &mut [u8]) -> usize {
        let serial = unsafe { self.get_inner() };
        let mut i = 0;
        for byte in data.iter_mut() {
            // TODO: Implement a timeout
            *byte = serial.receive();
            i += 1;
        }
        i
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        send_string(s);
        Ok(()) // Return Ok to indicate success
    }
}
