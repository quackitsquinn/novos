use core::fmt::Write;

use cake::{
    Mutex, Once, info,
    log::{self, Log},
};
use uart_16550::SerialPort;

static SERIAL: Once<Mutex<SerialPort>> = Once::new();

const SERIAL_PORT_NUM: u16 = 0x3F8; // Default COM1 port
const LOG_LEVEL: log::Level = log::Level::Trace;

pub fn init() {
    SERIAL.call_once(|| {
        let port = unsafe { SerialPort::new(SERIAL_PORT_NUM) };
        Mutex::new(port)
    });

    log::set_logger(&LogAdapter).expect("Failed to set logger");
    log::set_max_level(LOG_LEVEL.to_level_filter());
    info!("Serial port initialized!")
}

pub fn is_init() -> bool {
    SERIAL.is_completed()
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let serial = SERIAL.get().expect("serial");
    if serial.is_locked() {
        // If the serial port is locked, we cannot write to it.
        return;
    }
    let mut serial = serial.lock();
    write!(serial, "{}", args).expect("Failed to write to serial port");
}

/// Serial print
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}
/// Serial print with newline
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\n"));
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!($fmt, "\n"), $($arg)*)
    };
}

struct LogAdapter;

impl Log for LogAdapter {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!(
                "TRAMPOLINE: [{}] {}:{} {}",
                record.level(),
                record.file().unwrap_or("?"),
                record.line().unwrap_or(0),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
