use std::{
    env,
    fs::File,
    io::{stdout, BufRead, Read, Write},
    process::Stdio,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub iso: String,
    pub wait_for_debugger: bool,
    pub graphics: bool,
    pub memory: String,
    pub serial: Vec<String>,
    pub dev_exit: bool,
    /// Different than wait_for_debugger, as it will actually connect to the kernel debug harness. Unimplemented.
    pub debug: bool,
}

impl Config {
    pub fn run(&self) {
        let mut args = vec!["-cdrom", &self.iso];
        if self.wait_for_debugger {
            args.push("-s");
            args.push("-S");
        }
        if !self.graphics {
            args.push("-nographic");
        }
        args.push("-m");
        args.push(&self.memory);
        for serial in &self.serial {
            args.push("-serial");
            args.push(serial);
        }
        if self.dev_exit {
            args.push("-device");
            args.push("isa-debug-exit,iobase=0xf4,iosize=0x04");
        }
        let status = std::process::Command::new("qemu-system-x86_64")
            .args(&args)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("qemu-system-x86_64 failed to start");

        let mut stdout = status.stdout.expect("Failed to get stdout");
        let stderr = status.stderr.expect("Failed to get stderr");

        let pty = find_pty(&mut stdout);

        spawn_out_handler(Box::new(stdout), "stdout", true);
        spawn_out_handler(Box::new(stderr), "stderr", true);
    }

    pub fn empty() -> Config {
        Config {
            iso: "".to_string(),
            wait_for_debugger: false,
            graphics: true,
            memory: "".to_string(),
            serial: Vec::new(),
            dev_exit: false,
            debug: false,
        }
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
        cfg
    }
}

fn spawn_out_handler(out: Box<dyn Read>, name: &str, print: bool) {
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

fn find_pty(reader: &mut dyn Read) -> Option<String> {
    let mut br = std::io::BufReader::new(reader);
    let mut buf = String::new();
    let line = br.read_line(&mut buf);
    if line.is_err() {
        return None;
    }
    let line = line.unwrap();
    if line == 0 {
        return None;
    }
    let line = buf.trim();
    if line.starts_with("Opening debug harness on pty: ") {
        return Some(line.split(": ").last().unwrap().to_string());
    }
    None
}
fn run_harness(ptr: &str) {}
