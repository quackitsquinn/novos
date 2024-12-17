use std::{
    env,
    fs::File,
    io::{stdout, BufRead, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

mod packet_handler;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub iso: String,
    pub wait_for_debugger: bool,
    pub graphics: bool,
    pub memory: String,
    pub serial: Vec<String>,
    pub dev_exit: bool,
    pub extra_args: Vec<String>,
}

impl Config {
    pub fn run(&mut self) {
        let mut args = vec!["-cdrom".to_string(), self.iso.to_string()];
        self.add_debug_flags(&mut args);
        self.add_memory(&mut args);
        self.add_serial_ports(&mut args);
        self.add_extra_args(&mut args);

        if env::var("VERBOSE").is_ok() {
            println!("Running qemu with args: {:?}", args);
        }

        let mut qemu = std::process::Command::new("qemu-system-x86_64")
            .args(&args)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("qemu-system-x86_64 failed to start");

        let mut stdout = qemu.stdout.take().expect("Failed to get stdout");
        let stderr = qemu.stderr.take().expect("Failed to get stderr");

        let pty: Option<(PathBuf, String)> = find_pty(&mut stdout);

        if let Some((pty, name)) = pty {
            if name == "serial0" {
                println!("Found serial0 pty: {:?}", pty);
                packet_handler::run(&pty);
            } else {
                eprintln!("Found unknown pty: {:?}", pty);
            }
        }

        spawn_out_handler(Box::new(stdout), "stdout", false);
        spawn_out_handler(Box::new(stderr), "stderr", false);
        qemu.wait().expect("Failed to wait for qemu");
    }

    pub fn empty() -> Config {
        Config {
            iso: "".to_string(),
            wait_for_debugger: false,
            graphics: true,
            memory: "".to_string(),
            serial: Vec::new(),
            dev_exit: false,
            extra_args: Vec::new(),
        }
    }

    fn add_debug_flags(&mut self, args: &mut Vec<String>) {
        if self.wait_for_debugger {
            args.push("-s".to_string());
            args.push("-S".to_string());
        }
        if self.dev_exit {
            args.push("-device".to_string());
            args.push("isa-debug-exit,iobase=0xf4,iosize=0x04".to_string());
        }
        if !self.graphics {
            args.push("-nographic".to_string());
            args.push("-monitor".to_string());
            args.push("pty".to_string());
        }
    }

    fn add_memory(&mut self, args: &mut Vec<String>) {
        args.push("-m".to_string());
        args.push(self.memory.clone());
    }

    fn add_serial_ports(&mut self, args: &mut Vec<String>) {
        for serial in &self.serial {
            args.push("-serial".to_string());
            args.push(serial.clone());
        }
    }

    fn add_extra_args(&mut self, args: &mut Vec<String>) {
        let extra_args = env::var("QEMU_ARGS").unwrap_or("".to_string());
        args.extend(extra_args.split_whitespace().map(|s| s.to_string()));
    }
}

impl Default for Config {
    /// Creates the default configuration based on the environment variables.
    fn default() -> Self {
        let debug_mode = env::var("DEBUG").is_ok();
        let no_display = env::var("NO_DISPLAY").is_ok();
        let kernel_mem = env::var("KERNEL_MEM").unwrap_or("1G".to_string());
        let iso = if debug_mode || no_display {
            "novos_debug.iso"
        } else {
            "novos.iso"
        };
        let iso_path = format!("target/artifacts/{}", iso);
        let mut cfg = Config::empty();
        cfg.iso = iso_path;
        cfg.memory = kernel_mem;
        cfg.dev_exit = no_display || debug_mode;
        cfg.graphics = !no_display;
        cfg.wait_for_debugger = debug_mode;
        if !no_display {
            // This breaks stuff if we don't have a display
            cfg.serial.push("stdio".to_string());
        }
        cfg.serial.push("pty".to_string());
        cfg
    }
}

fn spawn_out_handler(out: Box<dyn Read + Send>, name: &str, print: bool) {
    let name = name.to_string();
    std::thread::spawn(move || spawn_out_handler_inner(out, name, print));
}

fn spawn_out_handler_inner(out: Box<dyn Read>, name: String, print: bool) {
    let mut br = std::io::BufReader::new(out);
    let mut f = File::create(format!("{}.log", name)).expect("Failed to create log file");
    // We would use a buffer writer, but having to flush it would be painful and probably won't increase performance in any meaningful way
    let mut buf = Vec::new();
    loop {
        let len = br.read_until(b'\n', &mut buf).expect("Failed to read line");
        if len == 0 {
            break;
        }
        if print {
            // Preserve whatever we read. Might be bad, but if the vm is spitting out garbage, we want to see it
            stdout()
                .lock()
                .write_all(&buf)
                .expect("Failed to write to stdout");
        }
        f.write_all(&buf).expect("Failed to write to file");
        buf.clear();
    }
}
/// Finds the pty path in the qemu output. In the format of (path, name)
fn find_pty(reader: &mut dyn Read) -> Option<(PathBuf, String)> {
    let mut br = std::io::BufReader::new(&mut *reader);
    let mut buf = String::new();
    let line = br.read_line(&mut buf);
    if line.is_err() {
        println!("Failed to read line: {:?}", line.err());
        return None;
    }
    let line = line.unwrap();
    if line == 0 {
        println!("No data read");
        return None;
    }
    let line = buf.trim();
    println!("Read line: {}", line);
    if line.starts_with("char device redirected to ") {
        let path = line
            .trim_start_matches("char device redirected to ")
            .split_whitespace()
            .next()?;
        if line.contains("monitor") {
            println!("Found monitor pty: {}", path);
            return find_pty(reader);
        }
        // char device redirected to /dev/ttys005 (label serial0)
        // serial0)
        // serial0
        let name = line.split_whitespace().last()?.trim_end_matches(')');
        return Some((PathBuf::from(path), name.to_string()));
    }
    None
}
