// Enable no_std if the std feature is not enabled
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub mod common;
