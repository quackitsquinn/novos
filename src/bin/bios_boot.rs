use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    str::Lines,
    thread,
};

pub fn main() {
    let mut command = Command::new("qemu-system-x86_64");
    command
        .arg("-cdrom")
        .arg("target/artifacts/novos.iso") // We don't use a bios specific iso because it supports both
        .arg("-serial")
        .arg("stdio")
        .arg("-serial")
        .arg("pty") // We don't use this, but it's here to prevent the default serial port from being used
        .arg("-m")
        .arg("1G");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    if env::var("DEBUG").is_ok() {
        println!("Running in debug mode");
        command.arg("-S").arg("-s");
    }

    let mut command = command.spawn().expect("qemu-system-x86_64 failed to start");
    let stdout = command.stdout.take().expect("Failed to get stdout");
    let stderr = command.stderr.take().expect("Failed to get stderr");
    thread::spawn(move || handle_stream(Box::new(stdout), "stdout", true));
    thread::spawn(move || handle_stream(Box::new(stderr), "stderr", false));
    command.wait().expect("qemu-system-x86_64 failed to run");
}

type Reader = Box<dyn Read>;

fn handle_stream(stream: Reader, name: &'static str, print: bool) {
    let mut br = BufReader::new(stream);
    let mut f = File::create(format!("{}.log", name)).expect("Failed to create log file");

    let mut lines = br.lines();
    while let Some(line) = lines.next() {
        let line = line.expect("Failed to get line");
        f.write_all(line.as_bytes())
            .expect("Failed to write to file");
        f.write_all(b"\n").expect("Failed to write to file");
        if print {
            println!("{}", line);
        }
    }
    unreachable!();
}
