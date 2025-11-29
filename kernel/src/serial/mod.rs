//! Serial port driver for debug output.
//!
//! This module is based off of the uart_16550 crate, which is a driver for the 16550 UART chip.

use core::convert::Infallible;


use crate::declare_module;

pub mod interface;
pub mod raw; // Things to interact with the serial port directly

declare_module!("serial", init);

fn init() -> Result<(), Infallible> {
    interface::init();

    Ok(())
}
