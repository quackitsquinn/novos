use bytemuck::{Pod, Zeroable};

use super::pod_checksum;

/// A packet that can be sent over the serial port.
#[derive(Debug, Clone, Copy, Zeroable)]
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
    pub fn new(command: u8, data: T) -> Self {
        let command_checksum = !(pod_checksum(&data).wrapping_add(command));
        Self {
            command,
            command_checksum,
            data,
        }
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

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn contained_checksum(&self) -> u8 {
        self.command_checksum
    }

    pub fn checksum(&self) -> u8 {
        pod_checksum(self)
    }
}

// Safety: u8 is Pod, and T is Pod, so Packet<T> is Pod
unsafe impl<T> Pod for Packet<T> where T: Pod {}
