use std::io::{self, Read, Write};

use bytemuck::Pod;

/// A marker trait for types that can be read from and written to, suitable for serial communication.
pub trait ReadWrite: Read + Write + 'static {}

impl<T> ReadWrite for T where T: Read + Write + 'static {}

pub(crate) struct SerialStream {
    inner: Box<dyn ReadWrite>,
}

impl SerialStream {
    pub fn new<T>(inner: T) -> Self
    where
        T: ReadWrite + 'static,
    {
        SerialStream {
            inner: Box::new(inner),
        }
    }

    #[inline(always)]
    pub fn get_inner(&mut self) -> &mut dyn ReadWrite {
        self.inner.as_mut()
    }

    pub fn read_ty<T: Pod + 'static>(&mut self) -> Result<T, io::Error> {
        // INFO: This would be faster if this could be an array, but apparently size_of::<T> can fail, so this can't happen.
        let mut buf = vec![0u8; std::mem::size_of::<T>()];
        self.inner.read_exact(&mut buf)?;
        Ok(*bytemuck::from_bytes(&buf))
    }
}
