//! Common utilities and types for kserial.
use bytemuck::Pod;

pub(crate) mod array_vec;
pub mod commands;
pub(crate) mod fixed_null_str;
pub(crate) mod macros;
pub(crate) mod packet;
pub(crate) mod validate;

pub use array_vec::ArrayVec;
pub use fixed_null_str::FixedNulString;
pub use packet::Packet;
pub use validate::Validate;

/// Signature for the start of a packet mode entry sequence.
pub const PACKET_MODE_ENTRY_SIG: [u8; 10] = *b"KSP\0\0ENTER";

/// Trait for types that can be sent as packet contents.
/// This includes a packet ID, size, and methods for checksum and conversion to a packet.
/// The packet ID must be specified manually.
pub trait PacketContents: Sized + Pod + Validate {
    /// The packet ID for this type.
    const ID: u8;
    /// The size of the type in bytes.
    const SIZE: usize = core::mem::size_of::<Self>();
    /// The size of the entire packet in bytes.
    const PACKET_SIZE: usize = core::mem::size_of::<Packet<Self>>();

    /// Calculate the checksum of the type.
    fn checksum(&self) -> u8 {
        pod_checksum(self)
    }

    /// Convert the type into a packet.
    fn into_packet(self) -> packet::Packet<Self> {
        unsafe { packet::Packet::new(Self::ID, self) }
    }
}

/// Calculate the checksum of a POD type.
/// The checksum is the sum of all bytes in the type, wrapping on overflow.
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
#[allow(unused)]
pub(crate) mod test_log {
    macro_rules! trace {
        ($($arg:tt)*) => {
            #[cfg(test)]
            {
                ::log::trace!($($arg)*);
            }
        };

    }
    macro_rules! debug {
        ($($arg:tt)*) => {
            #[cfg(test)]
            {
                ::log::debug!($($arg)*);
            }
        };
    }
    macro_rules! info {
        ($($arg:tt)*) => {
            #[cfg(test)]
            {
                ::log::info!($($arg)*);
            }
        };
    }
    macro_rules! warn {
        ($($arg:tt)*) => {
            #[cfg(test)]
            {
                ::log::warn!($($arg)*);
            }
        };
    }
    macro_rules! error {
        ($($arg:tt)*) => {
            #[cfg(test)]
            {
                ::log::error!($($arg)*);
            }
        };
    }

    pub(crate) use info;

    #[cfg(test)]
    mod log_internal {

        use ctor::ctor;

        #[ctor]
        static INIT: () = {
            env_logger::builder().is_test(true).init();
        };
    }
}
