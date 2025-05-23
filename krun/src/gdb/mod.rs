use std::{env::VarError, fs, io::ErrorKind, option, path::PathBuf};

use toml::{Value, map::Map};
use which::which;

mod cfg;
mod invocation;

pub struct GdbInstance {
    gdb: std::process::Child,
}
