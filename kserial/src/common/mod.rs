use bytemuck::{Pod, Zeroable};

/// A packet that can be sent over the serial port.
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct Packet<T>
where
    T: Pod,
{
    pub command: u8,
    pub command_checksum: u8,
    pub data: T,
}

// Safety: u8 is Pod, and T is Pod, so Packet<T> is Pod
unsafe impl<T> Pod for Packet<T> where T: Pod {}

pub mod array_vec;
pub mod commands;
pub mod fixed_null_str;

pub trait PacketContents: Sized {
    const ID: u8;
    const SIZE: usize = core::mem::size_of::<Self>();
}

pub(crate) fn pod_checksum<T>(data: &T) -> u8
where
    T: Pod,
{
    let bytes = bytemuck::bytes_of(data);
    let mut checksum: u8 = 0;
    for &byte in bytes.iter() {
        checksum = checksum.wrapping_add(byte);
    }
    checksum
}
