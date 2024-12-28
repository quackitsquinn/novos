pub mod serial_adapter;

use serial_adapter::write_wrapper;
pub use serial_adapter::{SerialAdapter, WouldBlockError};
use spin::Once;

use crate::common::commands::Command;
use core::fmt::Write;

static SERIAL_ADAPTER: Once<&'static dyn SerialAdapter> = Once::new();

pub fn init(adapter: &'static dyn SerialAdapter) {
    SERIAL_ADAPTER.call_once(|| adapter);
}
#[macro_export]
macro_rules! string_precondition {
    ($str: expr) => {
        if $str.contains('\0') {
            panic!("String contains null byte");
        }
    };
}

impl Command<'_> {
    pub fn send(&self) {
        let adapter = SERIAL_ADAPTER
            .get()
            .expect("Serial adapter not initialized");
        adapter.send(self.id());
        match *self {
            Command::WriteString(s) => write_string(*adapter, s),
            Command::WriteArguments(args) => write_arguments(*adapter, args),
            Command::SendFile(filename, contents) => send_file(*adapter, filename, contents),
            Command::DisablePacketSupport => {}
        }
    }
}

fn write_string<T>(a: &T, s: &str)
where
    T: SerialAdapter + ?Sized,
{
    string_precondition!(s);
    a.send_slice(s.as_bytes());
    a.send(0);
}

fn write_arguments<T>(a: &T, args: &core::fmt::Arguments)
where
    T: SerialAdapter + ?Sized,
{
    write_wrapper(a, args);
    a.send(0);
}

fn send_file<T>(a: &T, filename: &str, contents: &[u8])
where
    T: SerialAdapter + ?Sized,
{
    a.send_slice(filename.as_bytes());
    a.send(0);

    let len = (contents.len() as u32).to_le_bytes();
    a.send_slice(&len);

    a.send_slice(contents);
}
