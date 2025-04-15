use bytemuck::{Pod, Zeroable};

use super::pod_checksum;

/// A packet that can be sent over the serial port.
#[derive(Debug, Clone, Copy, Zeroable)]
#[repr(C)]
pub struct Packet<T>
where
    T: Pod,
{
    command: u8,
    command_checksum: u8,
    data: T,
}

impl<T> Packet<T>
where
    T: Pod,
{
    pub unsafe fn new(command: u8, data: T) -> Self {
        let mut no_chk = Self {
            command,
            command_checksum: 0,
            data,
        };
        let checksum = pod_checksum(&no_chk);
        no_chk.command_checksum = (!checksum).wrapping_add(1);
        no_chk
    }

    pub unsafe fn from_raw_parts_unchecked(command: u8, command_checksum: u8, data: T) -> Self {
        Self {
            command,
            command_checksum,
            data,
        }
    }

    pub fn from_raw_parts(command: u8, command_checksum: u8, data: T) -> Option<Self> {
        let new = Self {
            command,
            command_checksum,
            data,
        };

        if new.checksum() != 0 {
            return None;
        }

        Some(new)
    }

    pub fn command(&self) -> u8 {
        self.command
    }

    pub fn payload(&self) -> &T {
        &self.data
    }

    pub fn contained_checksum(&self) -> u8 {
        self.command_checksum
    }

    pub fn checksum(&self) -> u8 {
        pod_checksum(self)
    }
    /// Validates the checksum of the packet.
    pub fn validate(&self) -> bool {
        self.checksum() == 0
    }
}

// Safety: u8 is Pod, and T is Pod, so Packet<T> is Pod
unsafe impl<T> Pod for Packet<T> where T: Pod {}

#[cfg(test)]
mod tests {
    use crate::common::{commands::StringPacket, PacketContents};

    #[test]
    fn test_checksum_correct() {
        let string_payload = StringPacket::new("Hello, world!").unwrap();
        let packet = string_payload.into_packet();
        assert_eq!(packet.checksum(), 0);
    }
}
