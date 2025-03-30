#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

pub mod client;

#[cfg(feature = "std")]
pub mod server;

pub mod common;
