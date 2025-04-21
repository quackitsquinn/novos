use bytemuck::Pod;
use packet::Packet;
use validate::Validate;

pub mod array_vec;
pub mod commands;
pub mod fixed_null_str;
pub(crate) mod macros;
pub mod packet;
pub mod validate;

// KSerial Packet \0\0 ENTER
// This should be distinct enough to avoid conflicts with anything else
pub const PACKET_MODE_ENTRY_SIG: [u8; 10] = *b"KSP\0\0ENTER";

pub trait PacketContents: Sized + Pod + Validate {
    const ID: u8;
    const SIZE: usize = core::mem::size_of::<Self>();
    const PACKET_SIZE: usize = core::mem::size_of::<Packet<Self>>();

    fn checksum(&self) -> u8 {
        pod_checksum(self)
    }

    fn into_packet(self) -> packet::Packet<Self> {
        unsafe { packet::Packet::new(Self::ID, self) }
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
