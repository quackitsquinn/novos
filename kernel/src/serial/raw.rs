#![allow(
    dead_code,
    reason = "This is a stub module that is only used on non-x86_64 targets"
)]
use core::{fmt, panic};

///! This is a module that wraps the uart_16550 crate for serial use.
///! The point of this module is to allow rls to work with the uart_16550 crate on non-x86_64 targets.
///! Because of how the uart_16550 crate is implemented, rls can't see the functions in the crate directly, so we have to wrap them in this module.

#[cfg(target_arch = "x86_64")]
pub type SerialPort = uart_16550::SerialPort;

#[cfg(not(target_arch = "x86_64"))]
pub type SerialPort = NoOpSerialPort;

// Copy the u16 from the uart_16550 crate
struct NoOpSerialPort(u16);

// The following is an almost direct copy and paste from the uart_16550 crate.
impl NoOpSerialPort {
    /// Base port.
    fn port_base(&self) -> u16 {
        panic!("Serial port not available");
    }

    /// Data port.
    ///
    /// Read and write.
    fn port_data(&self) -> u16 {
        self.port_base()
    }

    /// Interrupt enable port.
    ///
    /// Write only.
    fn port_int_en(&self) -> u16 {
        self.port_base() + 1
    }

    /// Fifo control port.
    ///
    /// Write only.
    fn port_fifo_ctrl(&self) -> u16 {
        self.port_base() + 2
    }

    /// Line control port.
    ///
    /// Write only.
    fn port_line_ctrl(&self) -> u16 {
        self.port_base() + 3
    }

    /// Modem control port.
    ///
    /// Write only.
    fn port_modem_ctrl(&self) -> u16 {
        self.port_base() + 4
    }

    /// Line status port.
    ///
    /// Read only.
    fn port_line_sts(&self) -> u16 {
        self.port_base() + 5
    }

    /// Creates a new serial port interface on the given I/O base port.
    ///
    /// This function is unsafe because the caller must ensure that the given base address
    /// really points to a serial port device and that the caller has the necessary rights
    /// to perform the I/O operation.
    pub const unsafe fn new(_base: u16) -> Self {
        panic!("Serial port not available");
    }

    /// Initializes the serial port.
    ///
    /// The default configuration of [38400/8-N-1](https://en.wikipedia.org/wiki/8-N-1) is used.
    pub fn init(&mut self) {
        panic!("Serial port not available");
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, _data: u8) {
        panic!("Serial port not available");
    }

    /// Sends a raw byte on the serial port, intended for binary data.
    pub fn send_raw(&mut self, _data: u8) {
        panic!("Serial port not available");
    }

    /// Tries to send a raw byte on the serial port, intended for binary data.
    pub fn try_send_raw(&mut self, _data: u8) -> Result<(), WouldBlockError> {
        panic!("Serial port not available");
    }

    /// Receives a byte on the serial port.
    pub fn receive(&mut self) -> u8 {
        panic!("Serial port not available");
    }

    /// Tries to receive a byte on the serial port.
    pub fn try_receive(&mut self) -> Result<u8, WouldBlockError> {
        panic!("Serial port not available");
    }
}

impl fmt::Write for NoOpSerialPort {
    fn write_str(&mut self, _s: &str) -> fmt::Result {
        panic!("Serial port not available");
    }
}

/// The `WouldBlockError` error indicates that the serial device was not ready immediately.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WouldBlockError;

impl fmt::Display for WouldBlockError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("Serial port not available");
    }
}
