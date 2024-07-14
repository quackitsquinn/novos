use std::process::Command;

pub fn main() {
    let mut command = Command::new("qemu-system-x86_64")
        .arg("-cdrom")
        .arg("target/artifacts/novos.iso") // We don't use a bios specific iso because it supports both
        .arg("-serial")
        .arg("stdio")
        .arg("-m")
        .arg("1G")
        .spawn()
        .expect("Failed to start qemu-system-x86_64");
    let _ = command
        .wait()
        .expect("qemu-system-x86_64 exited with an error");
}
