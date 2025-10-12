//! Serial port driver for debug output.
//!
//! This module is based off of the uart_16550 crate, which is a driver for the 16550 UART chip.

use core::convert::Infallible;

use cake::log::{self, Level, Log, Metadata, Record};
use kproc::log_filter;

use crate::{declare_module, mp, println};

pub mod interface;
pub mod raw; // Things to interact with the serial port directly

/// The log level for the serial port.
pub const LOG_LEVEL: Level = log_level();

struct SerialLog;

impl Log for SerialLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut target = record.metadata().target();
            if let Some((i, _)) = target.rmatch_indices("::").skip(1).next() {
                target = &target[i + 2..];
            }

            if !log_filter!(target) {
                return;
            }

            let mut file = record.file().unwrap_or("unknown");
            if let Some((i, _)) = file.rmatch_indices("/").skip(1).next() {
                file = &file[i + 1..];
            }

            let line = record.line().unwrap_or(0);
            let core = mp::current_core_id();
            // [level] target[core] file:line message
            println!(
                "[{}] {}[{}] {}:{} {}",
                record.level(),
                target,
                core,
                file,
                line,
                record.args()
            );

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

const fn log_level() -> Level {
    // TODO: option_env!("LOG_LEVEL")
    Level::Trace
}
