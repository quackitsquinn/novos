use core::str;

use bytemuck::{Pod, Zeroable};
use kserial_derive::Validate;

use crate::common::{
    array_vec::{varlen, ArrayVec},
    PacketContents,
};

use super::ids::STRING_PACKET_ID;

/// A command to send a string packet.
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq, Validate)]
#[repr(C)]
pub struct StringPacket {
    /// The string data.
    pub data: varlen!(u8, StringPacket::CAPACITY),
}

impl PacketContents for StringPacket {
    const ID: u8 = STRING_PACKET_ID;
}

impl StringPacket {
    /// The maximum capacity of the string packet.
    pub const CAPACITY: usize = 32;

    /// Create a new `StringPacket` command.
    pub fn new(data: &str) -> Option<Self> {
        let data = ArrayVec::from_str(data)?;
        Some(Self { data })
    }

    /// Create a `StringPacket` from raw bytes without validation.
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        // Safety: The caller must ensure that the bytes are valid.
        // Also, this is a relatively no-op operation, so it's safe to mark as unsafe.
        let data = unsafe { ArrayVec::from_bytes_unchecked(bytes) };
        Self { data }
    }

    /// Get the string data as a `&str`.
    pub fn as_str(&self) -> &str {
        // Safety: The contained bytes are guaranteed to be valid UTF-8.
        unsafe { str::from_utf8_unchecked(&self.data) }
    }
}

impl AsRef<str> for StringPacket {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::fmt::Display for StringPacket {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_packet() {
        let packet = StringPacket::new("Hello, world!").unwrap();
        assert_eq!(packet.as_ref(), "Hello, world!");
        assert_eq!(packet.data.len(), 13);
        assert_eq!(&(&*packet.data)[0..13], (&b"Hello, world!").as_slice());
    }

    #[test]
    fn test_to_str() {
        let packet = StringPacket::new("Hello, world!").unwrap();
        let s: &str = packet.as_str();
        assert_eq!(s, "Hello, world!");
    }
}
