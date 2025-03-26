use core::fmt::Write;

/// An adapter for serial communication. This is used to abstract the serial port from the rest of the kernel.
pub trait SerialAdapter
where
    Self: Send + Sync,
{
    /// Send a byte over the serial port. Returns Some if sending the byte would block, None otherwise.
    fn send(&self, data: u8);
    /// Send a slice of bytes over the serial port. Returns Some if sending the slice would block, None otherwise.
    fn send_slice(&self, data: &[u8]);
    /// Read a byte from the serial port. Returns Some if reading the byte would block, None otherwise.
    fn read(&self) -> u8;
    /// Read a slice of bytes from the serial port. Returns Some if reading the slice would block, None otherwise.
    fn read_slice(&self, data: &mut [u8]) -> usize;
}
