//! nmm - Novos Memory Manager Library
#![cfg_attr(not(test), no_std)]

#[cfg(not(feature = "x86_64"))]
compile_error!("Only x86_64 architecture is currently supported.");

pub mod arch;
pub mod paging;
