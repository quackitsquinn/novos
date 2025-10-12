use core::fmt::Write;

use kserial::client::{SerialAdapter, get_serial_client, send_string};

use crate::serial::raw::SerialPort;

#[derive(Debug)]
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
}

impl SerialAdapter for Serial {
    fn send(&mut self, data: u8) {
        self.port.send_raw(data);
    }

    fn send_slice(&mut self, data: &[u8]) {
        for byte in data {
            self.port.send(*byte);
        }
    }

    fn read(&mut self) -> u8 {
        self.port.receive()
    }

    fn read_slice(&mut self, data: &mut [u8]) -> usize {
        let mut i = 0;
        for byte in data.iter_mut() {
            *byte = self.port.receive();
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
