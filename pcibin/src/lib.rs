#![cfg_attr(feature = "no_std", no_std)]

#[cfg(all(feature = "no_std", feature = "generate"))]
compile_error!("Cannot enable both 'default' and 'generate' features at the same time.");

#[cfg(feature = "generate")]
mod generate;

#[cfg(feature = "parse")]
mod parse;
