use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    str::Lines,
    thread,
};

use lazy_static::lazy_static;
use regex::Regex;
// the last capture group is the path. (not port num cuz some oses use \dev\ttysxxx and some use \dev\pts\x)
const PORT_REGEX_RAW: &str = r"char device redirected to (\S*)";

lazy_static! {
    static ref PORT_REGEX: Regex = Regex::new(PORT_REGEX_RAW).unwrap();
}

type Reader = Box<dyn Read>;

pub fn main() {
    let debug_mode = env::var("DEBUG").is_ok();

    let iso = if debug_mode {
        "novos_debug.iso"
    } else {
        "novos.iso"
    };

    let iso_path = format!("target/artifacts/{}", iso);

    let mut command = Command::new("qemu-system-x86_64");
    command
        .arg("-cdrom")
        .arg(&iso_path)
        .arg("-serial") // The log serial port
        .arg("stdio")
        .arg("-serial") // The debug harness serial port
        .arg("pty")
        .arg("-m")
        .arg("1G");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    if debug_mode {
        println!("Running in debug mode");
        command.arg("-S").arg("-s");
    }
    let mut command = command.spawn().expect("qemu-system-x86_64 failed to start");
    let stdout = command.stdout.take().expect("Failed to get stdout");
    let stderr = command.stderr.take().expect("Failed to get stderr");
    thread::spawn(move || stream_handler(Box::new(stdout), true, "stdout"));
    thread::spawn(move || stream_handler(Box::new(stderr), false, "stderr"));
    command.wait().expect("qemu-system-x86_64 failed to run");
}

fn stream_handler(stream: Reader, find_harness: bool, name: &'static str) {
    let mut br = BufReader::new(stream);
    let mut f = File::create(format!("{}.log", name)).expect("Failed to create log file");

    if find_harness {
        let pty = get_harness_pty(&mut br).expect("Failed to get pty");
        println!("Opening debug harness on pty: {}", pty);
        // Open a thread for the debug harness
        thread::spawn(move || open_harness(&pty));
    }
    let mut lines = br.lines();
    while let Some(line) = lines.next() {
        let line = line.expect("Failed to get line");
        f.write_all(line.as_bytes())
            .expect("Failed to write to file");
        f.write_all(b"\n").expect("Failed to write to file");
        //println!("{}", line.expect("Failed to get line"));
    }
}

fn open_harness(pty: &str) {
    let mut opts = OpenOptions::new()
        .read(true)
        .write(true)
        .open(pty)
        .expect("Failed to open pty");

    // Do the handshake
    println!("Connecting to debug harness");
    opts.write_all(b"1").expect("Failed to write to pty");
    println!("Sent handshake.. waiting for response");
    let mut out = [0; 1];
    opts.read_exact(&mut out).expect("Failed to read from pty");
    println!("Received response");
    println!("Debug harness connected with code: {}", out[0]);
}

fn get_harness_pty(br: &mut BufReader<Box<dyn Read>>) -> Option<String> {
    let mut stream_line = String::new();
    br.read_line(&mut stream_line).expect("Failed to read line");
    if let Some(mch) = PORT_REGEX.captures(&stream_line) {
        let port = mch.get(1).expect("Failed to get port").as_str();
        Some(port.to_string())
    } else {
        None
    }
}
