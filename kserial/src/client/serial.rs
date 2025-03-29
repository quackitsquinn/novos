use core::ops::Deref;

use spin::Once;

use super::{cfg, SerialAdapter};

pub struct Serial {
    adapter: Once<&'static dyn SerialAdapter>,
}

impl Serial {
    pub const fn new() -> Self {
        Self {
            adapter: Once::new(),
        }
    }

    pub fn init(&self, adapter: &'static dyn SerialAdapter) {
        self.adapter.call_once(|| adapter);
    }

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

    pub fn get(&self) -> Option<&'static dyn SerialAdapter> {
        self.adapter.get().map(|a| *a)
    }
}

#[cfg(test)]
pub mod tests {
    use core::pin::Pin;

    use super::*;
    use crate::client::serial_adapter::tests::TestSerialAdapter;

    pub struct TestSerialWrapper {
        pub serial: Serial,
        adapter: Pin<Box<TestSerialAdapter>>,
    }

    impl TestSerialWrapper {
        pub fn new() -> Self {
            let adapter = Box::pin(TestSerialAdapter::new());
            // SAFETY: adapter is pinned and will be dropped after serial.
            let adapter_ptr = unsafe { &*(adapter.as_ref().get_ref() as *const _) };
            let serial = Serial::new();
            serial.init(&*adapter_ptr);
            Self { serial, adapter }
        }

        pub fn get_adapter(&self) -> Pin<&TestSerialAdapter> {
            self.adapter.as_ref()
        }
    }

    impl Deref for TestSerialWrapper {
        type Target = Serial;

        fn deref(&self) -> &Self::Target {
            &self.serial
        }
    }
}
