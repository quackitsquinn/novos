use std::{env, process::Command};

fn main() {
    let mut command = Command::new("qemu-system-x86_64");
    command
        .arg("-cdrom")
        .arg("target/artifacts/kernel_tests.iso") // We don't use a bios specific iso because it supports both
        .arg("-serial")
        .arg("stdio")
        .arg("-m")
        .arg("1G");

    if env::var("DEBUG").is_ok() {
        println!("Running in debug mode");
        command.arg("-S").arg("-s");
    }
    let mut command = command.spawn().expect("qemu-system-x86_64 failed to start");
    command.wait().expect("qemu-system-x86_64 failed to run");
}
