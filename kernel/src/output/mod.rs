//! Kernel output module.
use core::{convert::Infallible, fmt::Write};

mod buf;

use arrayvec::ArrayString;
pub use buf::{FlushError, OutputBuffer};
use cake::{
    declare_module,
    log::{self, Level, Log, Metadata, Record},
};
use kproc::log_filter;

use crate::{
    display::{self, TERMINAL},
    interrupts::without_interrupts,
    mp,
};

/// The global standard I/O output buffer.
pub static STDIO: OutputBuffer<KernelWriter, 0x2000> = OutputBuffer::new(KernelWriter);

/// The log level for the serial port.
pub const LOG_LEVEL: Level = log_level();

/// A writer that writes to the kernel serial port and terminal.
#[derive(Clone, Copy, Debug)]
pub struct KernelWriter;

impl Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut ks = kserial::client::writer();
        ks.write_str(s)?;
        if display::is_initialized()
            && let Some(mut terminal) = TERMINAL.try_get()
        {
            write!(*terminal, "{}", s).unwrap();
        }
        Ok(())
    }
}

/// Prints to the terminal. Same functionality as the standard print! macro.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::output::_print(format_args!($($arg)*)));
}

/// Prints to the terminal, appending a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

/// Prints the given value and its source location then returns the value.
#[macro_export]
macro_rules! dbg {
    () => {
        $crate::println!("[{}:{}:{}]", core::file!(), core::line!(), core::column!());
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::println!("[{}:{}:{}] {} = {:#?}",
                    core::file!(), core::line!(), core::column!(),
                    core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;

    let mut buf: ArrayString<0x1000> = ArrayString::new();

    write!(&mut buf, "{}", args).unwrap();

    without_interrupts(|| {
        STDIO.push(&buf);
        let _ = STDIO.flush();
    });
}

impl Log for KernelWriter {
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

            STDIO.flush().ok();

            return;
        }
    }

    fn flush(&self) {
        STDIO.flush().ok();
    }
}

fn init() -> Result<(), Infallible> {
    // ZSTs are weird so this is fine.
    log::set_logger(&KernelWriter).unwrap();
    log::set_max_level(LOG_LEVEL.to_level_filter());
    Ok(())
}

declare_module!("output", init);

const fn log_level() -> Level {
    // TODO: option_env!("LOG_LEVEL")
    Level::Trace
}
