// Enable no_std if the std feature is not enabled
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub mod common;
