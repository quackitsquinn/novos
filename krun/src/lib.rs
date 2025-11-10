//! Custom QEMU control and GDB integration for running kernels.
mod env;
mod gdb;
mod qemu;

pub use qemu::{
    QemuConfig,
    chardev::{CharDev, CharDevRef},
};
