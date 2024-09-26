use std::{
    env,
    fs::{File, OpenOptions},
    io::{stdout, BufRead, BufReader, BufWriter, Read, Write},
    mem::MaybeUninit,
    process::{Child, Command, Stdio},
    str::Lines,
    sync::{Arc, Mutex},
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
    // This is bad. We can't use a Arc<Mutex<Child>> because `command.wait()` will block the thread, so we are stuck with this bad code.
    // .. well.. not stuck. TODO: use a loop to check if the child is alive so we can use a Arc<Mutex<Child>>.
    let child_ptr = &mut command as *mut _ as usize;
    ctrlc::set_handler(move || {
        let command = unsafe { &mut *(child_ptr as *mut Child) };
        command.kill().expect("Failed to kill qemu-system-x86_64");
        let mut buf_wrs = FILE_WRITERS.lock().unwrap();
        for buf_wr in buf_wrs.iter_mut() {
            buf_wr.flush().expect("Failed to flush");
        }
        FILE_WRITERS.lock().unwrap().clear();
    });
    command.wait().expect("qemu-system-x86_64 failed to run");
}

type Reader = Box<dyn Read>;

static FILE_WRITERS: Mutex<Vec<BufWriter<File>>> = Mutex::new(Vec::new());

fn handle_stream(stream: Reader, name: &'static str, print: bool) {
    let mut br = BufReader::new(stream);
    let mut bw =
        BufWriter::new(File::create(format!("{}.log", name)).expect("Failed to create log file"));

    let mut buf_wrs = FILE_WRITERS.lock().unwrap();
    buf_wrs.push(bw);

    let index = buf_wrs.len() - 1;

    drop(buf_wrs);

    let mut lines = br.lines();
    while let Some(line) = lines.next() {
        let mut line = line.expect("Failed to get line");
        line.push('\n');
        FILE_WRITERS.lock().unwrap()[index]
            .write_all(line.as_bytes())
            .expect("Failed to write");
        if print {
            loop {
                match stdout().lock().write_all(line.as_bytes()) {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            }
        }
    }
    // If we reach this point, the stream has ended. We don't need to flush because the ctrlc handler will flush all the buffers.
}
