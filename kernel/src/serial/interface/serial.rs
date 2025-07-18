use core::fmt::Write;

use cake::Mutex;
use kserial::client::{get_serial_client, send_string, SerialAdapter};

use crate::serial::raw::SerialPort;

pub struct Serial {
    port: Mutex<SerialPort>,
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
            port: Mutex::new(port),
        }
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
        self.port.lock().send(data);
    }

    pub unsafe fn send_slice_raw(&mut self, data: &[u8]) {
        for byte in data {
            unsafe { self.send_raw(*byte) };
        }
    }

    pub fn has_packet_support(&self) -> bool {
        true
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        send_string(s);
        Ok(()) // Return Ok to indicate success
    }
}

impl SerialAdapter for Serial {
    fn send(&self, data: u8) {
        self.port.lock().send(data);
    }

    fn send_slice(&self, data: &[u8]) {
        for byte in data {
            self.send(*byte);
        }
    }

    fn read(&self) -> u8 {
        self.port.lock().receive()
    }

    fn read_slice(&self, data: &mut [u8]) -> usize {
        let mut count = 0;
        for byte in data.iter_mut() {
            *byte = self.read();
            count += 1;
        }
        count
    }
}
