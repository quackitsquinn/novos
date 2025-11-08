//! Rust abstractions for configuring and running QEMU.
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread::{self, spawn},
    time::Duration,
};

use lazy_static::lazy_static;
use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};

use crate::{
    env::{self, qemu_path},
    gdb::{GdbConfig, run_gdb},
    packet::run_kserial,
    qemu::{
        chardev::{CharDev, CharDevRef},
        controller::QemuCtl,
    },
};

pub mod chardev;
pub mod controller;

/// Configuration for running QEMU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuConfig {
    /// Path to the ISO image to boot.
    pub iso: PathBuf,
    /// Debugger status.
    pub debugger: DebugStatus,
    /// Whether to display the QEMU window.
    pub display: bool,
    /// Amount of memory to allocate to the VM.
    pub memory: String,
    /// Serial port configurations for COM0.
    pub com0: Option<CharDevRef>,
    /// Serial port configurations for the qemu monitor.
    pub monitor: Option<CharDevRef>,
    /// Character devices to create.
    pub character_devices: Vec<CharDev>,
    /// Whether to enable the dev exit device.
    pub dev_exit: bool,
    /// Path to the UEFI code and variables images
    pub uefi_img: Option<(PathBuf, PathBuf)>,
    /// Extra arguments to pass to QEMU.
    pub extra_args: Vec<String>,
}

lazy_static! {
    /// Directory to store UEFI images.
    pub static ref UEFI_IMAGE_CACHE_DIR: &'static Path = Path::new("target/uefi");

    /// Path to the communication socket for kserial.
    pub static ref COMMUNICATION_SOCKET_PATH:  &'static Path= Path::new("target/com0.sock");
}

impl QemuConfig {
    /// Run QEMU with the current configuration.
    pub fn run(&mut self) {
        let mut args = vec!["-cdrom".to_string(), self.iso.display().to_string()];
        self.text_devices(&mut args);
        self.add_debug_flags(&mut args);
        self.add_memory(&mut args);
        self.uefi(&mut args);

        args.extend(self.extra_args.iter().cloned());

        if env::verbose_mode() {
            println!("QEMU Invocation: qemu-system-x86_64 {}", args.join(" "));
        }

        let qemu = Command::new(qemu_path())
            .args(&args)
            .spawn()
            .expect("qemu-system-x86_64 failed to start");

        let qemu = QemuCtl::new(qemu, &COMMUNICATION_SOCKET_PATH);

        let kserial_handle = spawn(move || run_kserial(qemu.clone()));

        let mut gdb = None;

        if self.debugger.present() && env::should_spawn_gdb() {
            // Sleep for a little bit so that QEMU has time to fail if the arguments are bad
            thread::sleep(Duration::from_millis(100));
            // If we're in debug mode, we want to wait for the debugger to attach
            gdb = Some(run_gdb(&mut GdbConfig::default()));
        }

        let _ = kserial_handle.join();
        if let Some(mut gdb) = gdb {
            // If we have a GDB instance, we need to wait for it to finish
            gdb.kill();
        }
    }

    /// Adds default character devices to the configuration.
    pub fn with_default_chardevs(mut self) -> Self {
        let com0 = CharDev::unix_socket("com0", &COMMUNICATION_SOCKET_PATH, None);
        let key = self
            .push_chardev(com0.clone())
            .expect("Failed to add com0 character device");
        self.com0(key);

        self
    }

    /// Create an empty QEMU configuration.
    pub fn empty() -> QemuConfig {
        QemuConfig {
            iso: PathBuf::new(),
            debugger: DebugStatus::NoDebug,
            display: true,
            memory: "".to_string(),
            com0: None,
            monitor: None,
            character_devices: Vec::new(),
            dev_exit: false,
            extra_args: Vec::new(),
            uefi_img: None,
        }
    }

    /// Adds the given character device to the configuration and appends the appropriate arguments to QEMU.
    /// Returns a reference to the character device that can be used in other parts of the configuration.
    ///
    /// Returns 'None` if the character device has a non-unique ID.
    pub fn push_chardev(&mut self, chardev: CharDev) -> Option<CharDevRef> {
        let dev_ref = chardev.dev_ref();
        if self.character_devices.iter().any(|d| d.id == chardev.id) {
            return None;
        }
        self.character_devices.push(chardev);
        Some(dev_ref)
    }

    /// Sets the character device for COM0.
    pub fn com0(&mut self, chardev: CharDevRef) {
        self.com0 = Some(chardev);
    }

    /// Sets the character device for the QEMU monitor.
    pub fn monitor(&mut self, chardev: CharDevRef) {
        self.monitor = Some(chardev);
    }

    /// Retrieves a character device by its ID.
    pub fn get_chardev_by_id(&self, id: &str) -> Option<&CharDev> {
        self.character_devices.iter().find(|d| d.id.as_ref() == id)
    }

    fn add_debug_flags(&mut self, args: &mut Vec<String>) {
        args.extend(self.debugger.to_flags().iter().map(|s| s.to_string()));
        if self.dev_exit {
            args.push("-device".to_string());
            args.push("isa-debug-exit,iobase=0xf4,iosize=0x04".to_string());
        }
        if !self.display {
            args.push("-nographic".to_string());
            args.push("-monitor".to_string());
            args.push("pty".to_string());
        }
    }

    fn add_memory(&mut self, args: &mut Vec<String>) {
        args.push("-m".to_string());
        args.push(self.memory.clone());
    }

    fn text_devices(&mut self, args: &mut Vec<String>) {
        for chardev in &self.character_devices {
            args.push("-chardev".to_string());
            args.push(chardev.as_parameter().to_string());
        }
        if let Some(com0) = &self.com0 {
            args.push("-serial".to_string());
            args.push("chardev:".to_string() + &com0.id());
        }

        if let Some(monitor) = &self.monitor {
            args.push("-monitor".to_string());
            args.push("chardev:".to_string() + &monitor.id());
        }
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
        let mut cfg = QemuConfig::empty();

        cfg.debugger = DebugStatus::from_env();

        if env::uefi_enabled() {
            cfg.uefi_img = Some(get_uefi_images());
        }

        if !env::display_enabled() {
            cfg.display = false;
        }

        cfg.iso = env::kernel_image_path();
        cfg.memory = env::memory_config();
        cfg.dev_exit = env::dev_exit_enabled();
        cfg.extra_args = env::extra_arguments();

        cfg
    }
}

fn get_uefi_images() -> (PathBuf, PathBuf) {
    let pre = Prebuilt::fetch(Source::LATEST, "target/uefi").unwrap();
    (
        pre.get_file(Arch::X64, FileType::Code),
        pre.get_file(Arch::X64, FileType::Vars),
    )
}

/// Debug status for QEMU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugStatus {
    /// No debug support.
    NoDebug,
    /// Enable GDB remote debugging and wait for debugger to attach.
    WaitForDebugger,
    /// Enable GDB remote debugging but do not wait for debugger to attach.
    Debugger,
}

impl DebugStatus {
    /// Create a DebugStatus from environment variables.
    pub fn from_env() -> Self {
        let (debug, wait) = crate::env::should_attach_debugger();

        if debug && wait {
            DebugStatus::WaitForDebugger
        } else if debug {
            DebugStatus::Debugger
        } else {
            DebugStatus::NoDebug
        }
    }

    /// Convert the DebugStatus to QEMU command line flags.
    pub fn to_flags(&self) -> &[&str] {
        match self {
            DebugStatus::NoDebug => &[],
            DebugStatus::Debugger => &["-s"],
            DebugStatus::WaitForDebugger => &["-s", "-S"],
        }
    }

    /// Check if a debugger might be present.
    pub fn present(&self) -> bool {
        match self {
            DebugStatus::NoDebug => false,
            DebugStatus::Debugger | DebugStatus::WaitForDebugger => true,
        }
    }
}
