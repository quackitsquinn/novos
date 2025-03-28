use bytemuck::{Pod, Zeroable};

pub mod array_vec;
pub mod commands;
pub mod fixed_null_str;
pub(crate) mod macros;
pub mod packet;

pub trait PacketContents: Sized + Pod {
    const ID: u8;
    const SIZE: usize = core::mem::size_of::<Self>();

    fn checksum(&self) -> u8 {
        pod_checksum(self)
    }

    fn into_packet(self) -> packet::Packet<Self> {
        packet::Packet::new(Self::ID, self)
    }
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
