//! Serial port driver for debug output.
//!
//! This module is based off of the uart_16550 crate, which is a driver for the 16550 UART chip.

use core::convert::Infallible;

use crate::{declare_module, println};

pub mod interface;
pub mod raw; // Things to interact with the serial port directly

pub const LOG_LEVEL: log::Level = log::Level::Trace;
pub const SERIAL_PORT_NUM: u16 = 0x3F8;
struct SerialLog;

impl log::Log for SerialLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let file = record.file().unwrap_or("?");
            let line = record.line().unwrap_or(0);
            println!("[{}] {}:{} {}", record.level(), file, line, record.args());
            return;
        }
    }

    fn flush(&self) {}
}

const LOGGER: SerialLog = SerialLog;

declare_module!("serial", init);

fn init() -> Result<(), Infallible> {
    interface::init();
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LOG_LEVEL.to_level_filter());
    Ok(())
}
