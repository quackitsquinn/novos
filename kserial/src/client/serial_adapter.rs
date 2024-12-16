/// An adapter for serial communication. This is used to abstract the serial port from the rest of the kernel.
pub trait SerialAdapter
where
    Self: Send + Sync,
{
    /// Send a byte over the serial port. Returns Some if sending the byte would block, None otherwise.
    fn send(&self, data: u8) -> Result<(), WouldBlockError>;
    /// Send a slice of bytes over the serial port. Returns Some if sending the slice would block, None otherwise.
    fn send_slice(&self, data: &[u8]) -> Result<(), WouldBlockError>;
    /// Read a byte from the serial port. Returns Some if reading the byte would block, None otherwise.
    fn read(&self) -> Result<u8, WouldBlockError>;
    /// Read a slice of bytes from the serial port. Returns Some if reading the slice would block, None otherwise.
    fn read_slice(&self, data: &mut [u8]) -> Result<usize, WouldBlockError>;

    /// Send a byte over the serial port. Blocks until the byte is sent.
    fn send_blocking(&self, data: u8) {
        while let Err(WouldBlockError) = self.send(data) {}
    }

    /// Send a slice of bytes over the serial port. Blocks until the slice is sent.
    fn send_slice_blocking(&self, data: &mut [u8]) {
        while let Err(WouldBlockError) = self.send_slice(data) {}
    }

    /// Read a byte from the serial port. Blocks until a byte is read.
    fn read_blocking(&self) -> u8 {
        loop {
            match self.read() {
                Ok(byte) => return byte,
                Err(WouldBlockError) => {}
            }
        }
    }

    /// Read a slice of bytes from the serial port. Blocks until the slice is read.
    fn read_slice_blocking(&mut self, data: &mut [u8]) -> usize {
        loop {
            match self.read_slice(data) {
                Ok(bytes) => return bytes,
                Err(WouldBlockError) => {}
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Would block")]
pub struct WouldBlockError;
