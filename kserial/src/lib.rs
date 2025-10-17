//! Serial communication library for kernel and user space.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod client;

#[cfg(feature = "std")]
pub mod server;

pub mod common;
