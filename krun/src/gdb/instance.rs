use std::{
    io::{PipeWriter, Read, Write, pipe, stdin, stdout},
    process::{Child, Command, exit},
    rc::Rc,
    thread,
    time::Duration,
};

use crate::gdb::{BINARY_PATH, GdbConfig, GdbInvocation};

pub struct Gdb(Child);

impl Gdb {
    fn wait_terminate(&mut self) {
        if let Err(e) = self.0.wait() {
            eprintln!("GDB process terminated with error: {}", e);
            exit(1);
        }
        exit(0);
    }
}

pub fn start_gdb(config: &GdbConfig, invocation: &GdbInvocation) -> Gdb {
    let mut command = invocation.build_command(vec![]);
    command.arg(BINARY_PATH);
    command.arg(format!(
        "--eval-command=target remote {}:{}",
        config.host, config.port
    ));
    let gdb = command.spawn().expect("Failed to start GDB process");

    Gdb(gdb)
}
