use std::{
    env,
    fs::File,
    io::{BufRead, Read, Write, stdout},
    path::PathBuf,
    process::{Command, Stdio},
    thread::{self, Thread, spawn},
};

use ovmf_prebuilt::Source;

use crate::{
    gdb::{GdbConfig, run_gdb},
    packet::run_kserial,
    qemu_ctl::QemuCtl,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuConfig {
    pub iso: String,
    pub wait_for_debugger: bool,
    pub graphics: bool,
    pub memory: String,
    pub serial: Vec<String>,
    pub dev_exit: bool,
    /// Path to the UEFI code and variables images
    pub uefi_img: Option<(PathBuf, PathBuf)>,
    pub extra_args: Vec<String>,
}

impl QemuConfig {
    pub fn run(&mut self) {
        let mut args = vec!["-cdrom".to_string(), self.iso.to_string()];
        self.add_debug_flags(&mut args);
        self.create_unix_chardev(
            &mut args,
            PathBuf::from("target/serial0.sock").to_str().unwrap(),
        );
        self.add_memory(&mut args);
        self.add_serial_ports(&mut args);
        self.add_extra_args(&mut args);
        self.uefi(&mut args);

        if env::var("VERBOSE").is_ok() {
            println!("QEMU Invocation: qemu-system-x86_64 {}", args.join(" "));
        }

        let qemu = Command::new("qemu-system-x86_64")
            .args(&args)
            .spawn()
            .expect("qemu-system-x86_64 failed to start");

        let qemu = QemuCtl::new(qemu, PathBuf::from("target/serial0.sock"));

        let thing = spawn(move || run_kserial(qemu.clone()));

        let mut gdb = None;
        if env::var("DEBUG").is_ok() {
            // If we're in debug mode, we want to wait for the debugger to attach
            gdb = Some(run_gdb(&mut GdbConfig::default()));
        }

        thing.join().expect("Failed to run kserial thread");
        if let Some(mut gdb) = gdb {
            // If we have a GDB instance, we need to wait for it to finish
            gdb.kill();
        }
    }

    pub fn empty() -> QemuConfig {
        QemuConfig {
            iso: "".to_string(),
            wait_for_debugger: false,
            graphics: true,
            memory: "".to_string(),
            serial: Vec::new(),
            dev_exit: false,
            extra_args: Vec::new(),
            uefi_img: None,
        }
    }

    fn add_debug_flags(&mut self, args: &mut Vec<String>) {
        if self.wait_for_debugger {
            args.push("-s".to_string());
            args.push("-S".to_string());
        }
        if self.dev_exit && env::var("NO_EXIT").is_err() {
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

    fn create_unix_chardev(&mut self, args: &mut Vec<String>, path: &str) {
        args.push("-chardev".to_string());
        args.push(format!("socket,path={},server=off,id=output", path));
    }

    fn uefi(&mut self, args: &mut Vec<String>) {
        if let Some(uefi_img) = &self.uefi_img {
            args.push("-drive".to_string());
            args.push(format!(
                "file={},format=raw,if=pflash",
                uefi_img.0.display()
            ));
            args.push("-drive".to_string());
            args.push(format!(
                "file={},format=raw,if=pflash",
                uefi_img.1.display()
            ));
        }
    }
}

impl Default for QemuConfig {
    /// Creates the default configuration based on the environment variables.
    fn default() -> Self {
        let debug_mode = env::var("DEBUG").is_ok();
        let no_display = env::var("NO_DISPLAY").is_ok();
        let kernel_mem = env::var("KERNEL_MEM").unwrap_or("1G".to_string());
        let iso_path = env::var("ISO").unwrap_or("boot_images/novaos.iso".to_string());
        let mut uefi_img = None;
        if env::var("USE_UEFI").is_ok() {
            let pre = ovmf_prebuilt::Prebuilt::fetch(Source::LATEST, "target/uefi").unwrap();
            uefi_img = Some((
                pre.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Code),
                pre.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Vars),
            ));
        }
        let mut cfg = QemuConfig::empty();
        cfg.iso = iso_path;
        cfg.memory = kernel_mem;
        cfg.dev_exit = no_display || debug_mode;
        cfg.graphics = !no_display;
        cfg.wait_for_debugger = debug_mode;
        cfg.uefi_img = uefi_img;
        if !no_display {
            // This breaks stuff if we don't have a display
            cfg.serial.push("stdio".to_string());
        }
        cfg.serial.push("pty".to_string());
        cfg
    }
}
