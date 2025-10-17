//! A client for serial communication.
use core::{fmt::Debug, mem, slice};

use bytemuck::Zeroable;
use spin::{Mutex, MutexGuard};

use crate::common::{packet::Packet, PacketContents, PACKET_MODE_ENTRY_SIG};

use super::{cfg, SerialAdapter};
/// A client for serial communication.
pub struct SerialClient<'a> {
    adapter: Mutex<Option<&'a mut dyn SerialAdapter>>,
}

impl<'a> SerialClient<'a> {
    /// Create a new, uninitialized serial client.
    pub const fn new() -> Self {
        Self {
            adapter: Mutex::new(None),
        }
    }

    /// Initialize the serial client with the given adapter.
    pub fn init(&self, adapter: &'a mut dyn SerialAdapter) {
        *self.adapter.lock() = Some(adapter);
    }

    /// Lock the serial client and get an adapter container.
    /// Returns None if the adapter is not initialized.
    pub fn lock(&'a self) -> Option<AdapterContainer<'a>> {
        AdapterContainer::new(self.adapter.lock())
    }

    /// Enable packet mode support. This will set the packet mode configuration option and send a marker over the serial connection.
    pub fn enable_packet_support(&'a self) {
        cfg::set_packet_mode(true);
        unsafe {
            self.lock()
                .expect("uninit")
                .send_pod(&PACKET_MODE_ENTRY_SIG); // Send a marker to indicate packet mode is enabled.
        }
    }
}

/// A container for a locked serial adapter.
pub struct AdapterContainer<'a> {
    adapter: MutexGuard<'a, Option<&'a mut dyn SerialAdapter>>,
}

impl<'a> AdapterContainer<'a> {
    fn new(
        guard: MutexGuard<'a, Option<&'a mut dyn SerialAdapter>>,
    ) -> Option<AdapterContainer<'a>> {
        if guard.is_some() {
            Some(Self { adapter: guard })
        } else {
            None
        }
    }

    /// Sends a packet over the serial connection.
    pub fn send_packet<T>(&mut self, data: &Packet<T>)
    where
        T: PacketContents,
    {
        if !cfg::should_output_serial() {
            return;
        }

        if !data.validate() {
            panic!("Invalid packet data");
        }

        unsafe {
            // TODO: This is a band-aid. I forgot about padding so raw-dogging the packet is not safe, so just send the fields independently.
            self.send_pod(&data.command());
            self.send_pod(&data.contained_checksum());
            self.send_pod(data.payload());
        }
    }

    /// Reads a packet from the serial connection. Validates the checksum and returns None if it is invalid.
    pub fn read_packet<T>(&mut self) -> Option<Packet<T>>
    where
        T: PacketContents,
    {
        if !cfg::should_input_serial() {
            return None;
        }
        let mut command = 0;
        let mut checksum = 0;
        let mut value = T::zeroed();
        unsafe {
            self.read_pod(&mut command);
            self.read_pod(&mut checksum);
            self.read_pod(&mut value);
        }

        let packet = Packet::from_raw_parts(command, checksum, value)?;

        if packet.validate() {
            Some(packet)
        } else {
            None
        }
    }

    /// Writes a POD type to the serial connection.
    /// This is *almost certainly* not the function you want to use. Use `send_packet` instead.
    pub unsafe fn send_pod<T>(&mut self, data: &T)
    where
        T: bytemuck::Pod,
    {
        if !cfg::should_output_serial() {
            return;
        }
        // We allow sending POD types over serial, even if we are not in packet mode.
        let adapter = self
            .adapter
            .as_mut()
            .expect("Serial adapter not initialized");
        let bytes = bytemuck::bytes_of(data);
        adapter.send_slice(bytes);
    }

    /// Reads a POD type from the serial connection.
    /// This is *almost certainly* not the function you want to use. Use `read_packet` instead.
    pub unsafe fn read_pod<T>(&mut self, dest: &mut T)
    where
        T: bytemuck::Pod,
    {
        // TODO: Is this good behavior? Probably not.
        if !cfg::should_input_serial() {
            return Zeroable::zeroed();
        }
        let adapter = self
            .adapter
            .as_mut()
            .expect("Serial adapter not initialized");
        let mut bytes =
            unsafe { slice::from_raw_parts_mut(dest as *mut T as *mut u8, mem::size_of::<T>()) };
        // TODO: Error type over panic
        assert_eq!(adapter.read_slice(&mut bytes), mem::size_of::<T>());
    }

    /// Get a mutable reference to the underlying adapter.
    pub fn get_adapter(&mut self) -> &mut MutexGuard<'a, Option<&'a mut dyn SerialAdapter>> {
        &mut self.adapter
    }
}

impl Debug for AdapterContainer<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AdapterContainer").finish()
    }
}

impl Debug for SerialClient<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SerialClient").finish()
    }
}

#[cfg(test)]
pub mod tests {
    use core::{ops::Deref, pin::Pin};

    use super::*;
    use crate::client::serial_adapter::tests::TestSerialAdapter;

    pub struct TestSerialWrapper<'a> {
        pub serial: SerialClient<'a>,
        adapter: Pin<Box<TestSerialAdapter>>,
    }

    impl<'a> TestSerialWrapper<'a> {
        pub fn new() -> Self {
            let mut adapter = Box::pin(TestSerialAdapter::new());
            // SAFETY: adapter is pinned and will be dropped after serial.
            let serial = SerialClient::new();
            serial.init(unsafe {
                mem::transmute::<&mut dyn SerialAdapter, &'a mut dyn SerialAdapter>(
                    &mut *adapter.as_mut() as &mut dyn SerialAdapter,
                )
            });
            Self { serial, adapter }
        }

        pub fn get_adapter(&self) -> Pin<&TestSerialAdapter> {
            self.adapter.as_ref()
        }
    }

    impl<'a> Deref for TestSerialWrapper<'a> {
        type Target = SerialClient<'a>;

        fn deref(&self) -> &Self::Target {
            &self.serial
        }
    }
}
