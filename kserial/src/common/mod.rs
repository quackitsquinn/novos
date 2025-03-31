use bytemuck::Pod;

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

    pub(crate) use {debug, error, info, trace};

    #[cfg(test)]
    mod log_internal {
        use std::io::{stdout, Write};

        use ctor::ctor;

        #[ctor]
        static INIT: () = {
            env_logger::builder().is_test(true).init();
        };
    }
}
