use std::{
    fs,
    process::{Child, exit},
};

use crate::gdb::{BINARY_PATH, GdbConfig, GdbInvocation};

pub struct Gdb(Child);

impl Gdb {
    pub fn wait_terminate(&mut self) {
        if let Err(e) = self.0.wait() {
            eprintln!("GDB process terminated with error: {}", e);
            exit(1);
        }
        exit(0);
    }

    pub fn kill(&mut self) {
        if let Err(e) = self.0.kill() {
            eprintln!("Failed to kill GDB process: {}", e);
        }
    }
}

fn get_gdb_scripts() -> Vec<String> {
    let mut scripts = vec![];
    let mut found_scratchpad = false;
    for file in fs::read_dir("gdb_scripts").unwrap() {
        let file = file.unwrap();
        if file.file_type().unwrap().is_file()
            && file.path().extension().map_or(false, |ext| ext == "gdb")
        {
            scripts.push(file.path().to_str().unwrap().to_string());
        }
        // Check for scratchpad.gdb (A .gitignore'd file for debugging)
        if file.file_name() == "scratchpad.gdb" {
            found_scratchpad = true;
        }
    }

    if !found_scratchpad {
        println!(
            "Creating scratchpad.gdb! This file is ignored by git, so you can use it for debugging without worrying about committing it."
        );
        fs::write("gdb_scripts/scratchpad.gdb", "").expect("Failed to create scratchpad.gdb");
        // Don't bother adding it to the list, as it will be created empty.
    }

    scripts.sort();
    scripts
}

pub fn start_gdb(config: &GdbConfig, invocation: &GdbInvocation) -> Gdb {
    let mut command = invocation.build_command(vec![]);
    command.arg(BINARY_PATH);
    command.arg(format!(
        "--eval-command=target remote {}:{}",
        config.host, config.port
    ));

    let scripts = get_gdb_scripts();

    for script in scripts {
        command.args(&["-x", &script]);
    }

    let gdb = command.spawn().expect("Failed to start GDB process");

    Gdb(gdb)
}
