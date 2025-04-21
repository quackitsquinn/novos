use core::{mem, slice};

use bytemuck::Zeroable;
use spin::Once;

use crate::common::{packet::Packet, PacketContents, PACKET_MODE_ENTRY_SIG};

use super::{cfg, SerialAdapter};

pub struct SerialClient {
    adapter: Once<&'static dyn SerialAdapter>,
}

impl SerialClient {
    pub const fn new() -> Self {
        Self {
            adapter: Once::new(),
        }
    }

    pub fn init(&self, adapter: &'static dyn SerialAdapter) {
        self.adapter.call_once(|| adapter);
    }

    /// Sends a packet over the serial connection.
    pub fn send_packet<T>(&self, data: &Packet<T>)
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
            self.send_pod(data);
        }
    }

    /// Reads a packet from the serial connection. Validates the checksum and returns None if it is invalid.
    pub fn read_packet<T>(&self) -> Option<Packet<T>>
    where
        T: PacketContents,
    {
        if !cfg::should_input_serial() {
            return None;
        }
        let mut value = Packet::zeroed();
        unsafe {
            self.read_pod(&mut value);
        }

        if value.validate() {
            Some(value)
        } else {
            None
        }
    }

    /// Writes a POD type to the serial connection.
    /// This is *almost certainly* not the function you want to use. Use `send_packet` instead.
    pub unsafe fn send_pod<T>(&self, data: &T)
    where
        T: bytemuck::Pod,
    {
        if !cfg::should_output_serial() {
            return;
        }
        // We allow sending POD types over serial, even if we are not in packet mode.
        let adapter = self.adapter.get().expect("Serial adapter not initialized");
        let bytes = bytemuck::bytes_of(data);
        adapter.send_slice(bytes);
    }

    /// Reads a POD type from the serial connection.
    /// This is *almost certainly* not the function you want to use. Use `read_packet` instead.
    pub unsafe fn read_pod<T>(&self, dest: &mut T)
    where
        T: bytemuck::Pod,
    {
        // TODO: Is this good behavior? Probably not.
        if !cfg::should_input_serial() {
            return Zeroable::zeroed();
        }
        let adapter = self.adapter.get().expect("Serial adapter not initialized");
        let mut bytes =
            unsafe { slice::from_raw_parts_mut(dest as *mut T as *mut u8, mem::size_of::<T>()) };
        // TODO: Error type over panic
        assert_eq!(adapter.read_slice(&mut bytes), mem::size_of::<T>());
    }

    pub fn get(&self) -> Option<&'static dyn SerialAdapter> {
        self.adapter.get().map(|a| *a)
    }

    pub fn enable_packet_support(&self) {
        cfg::set_packet_mode(true);
        unsafe {
            self.send_pod(&PACKET_MODE_ENTRY_SIG); // Send a marker to indicate packet mode is enabled.
        }
    }
}

#[cfg(test)]
pub mod tests {
    use core::{ops::Deref, pin::Pin};

    use super::*;
    use crate::client::serial_adapter::tests::TestSerialAdapter;

    pub struct TestSerialWrapper {
        pub serial: SerialClient,
        adapter: Pin<Box<TestSerialAdapter>>,
    }

    impl TestSerialWrapper {
        pub fn new() -> Self {
            let adapter = Box::pin(TestSerialAdapter::new());
            // SAFETY: adapter is pinned and will be dropped after serial.
            let adapter_ptr = unsafe { &*(adapter.as_ref().get_ref() as *const _) };
            let serial = SerialClient::new();
            serial.init(&*adapter_ptr);
            Self { serial, adapter }
        }

        pub fn get_adapter(&self) -> Pin<&TestSerialAdapter> {
            self.adapter.as_ref()
        }
    }

    impl Deref for TestSerialWrapper {
        type Target = SerialClient;

        fn deref(&self) -> &Self::Target {
            &self.serial
        }
    }
}
