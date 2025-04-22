use std::{
    fmt,
    io::{self, Read, Write},
};

use bytemuck::Pod;

use crate::common::{packet::Packet, PacketContents};

use super::handle_invalid_checksum;

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

    pub(crate) fn read_packet<C: PacketContents + fmt::Debug>(
        &mut self,
        cmd_id: u8,
    ) -> Result<Packet<C>, io::Error> {
        let checksum = unsafe { self.read_ty::<u8>()? };
        let packet = unsafe { self.read_ty::<C>()? };
        // //   if TypeId::of::<C>() == TypeId::of::<WriteFile>() {
        // println!("Received packet: {packet:?}");
        // println!("As bytes: {:?}", bytemuck::bytes_of(&packet));
        // //  }
        let full = Packet::from_raw_parts(cmd_id, checksum, packet)
            .ok_or_else(|| handle_invalid_checksum(packet.checksum()))?;
        Ok(full)
    }

    pub(crate) fn write_packet<C: PacketContents + fmt::Debug>(
        &mut self,
        packet: &Packet<C>,
    ) -> Result<(), io::Error> {
        if !packet.validate() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid packet data",
            ));
        }
        unsafe {
            self.write_ty::<u8>(&packet.command())?;
            self.write_ty::<u8>(&packet.contained_checksum())?;
            self.write_ty::<C>(&packet.payload())?;
        }
        Ok(())
    }

    pub unsafe fn read_ty<T: Pod + 'static>(&mut self) -> Result<T, io::Error> {
        // INFO: This would be faster if this could be an array, but apparently size_of::<T> can fail, so this can't happen.
        let mut buf = vec![0u8; std::mem::size_of::<T>()];
        self.inner.read_exact(&mut buf)?;
        Ok(*bytemuck::from_bytes(&buf))
    }

    pub unsafe fn write_ty<T: Pod + 'static>(&mut self, data: &T) -> Result<(), io::Error> {
        let bytes = bytemuck::bytes_of(data);
        self.inner.write_all(bytes)?;
        Ok(())
    }
}
