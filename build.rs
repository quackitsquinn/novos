use std::{
    fs,
    path::{Path, PathBuf},
};

use kbuild::config::{Config, ConfigBuilder};

fn main() {
    let kernel_dir = std::env::var("CARGO_BIN_FILE_KERNEL").expect("CARGO_BIN_FILE_KERNEL not set");
    let trampoline_dir =
        std::env::var("CARGO_BIN_FILE_TRAMPOLINE").expect("CARGO_BIN_FILE_TRAMPOLINE not set");
    let dbg_mode = std::env::var("DEBUG").is_ok();

    let limine_cfg = if dbg_mode {
        PathBuf::try_from("boot_cfg/main_debug.conf")
            .expect("Failed to convert boot_cfg/main_debug.conf to PathBuf")
    } else {
        PathBuf::try_from("boot_cfg/main.conf")
            .expect("Failed to convert boot_cfg/main.conf to PathBuf")
    };
    let _ = fs::create_dir("boot_images");

    let artifact_dir = PathBuf::from("target/artifacts");
    let kernel_binary = PathBuf::from(kernel_dir);
    let trampoline_binary = PathBuf::from(trampoline_dir);
    let iso_root = PathBuf::from("boot_images");
    let iso_name = "novaos.iso".to_string();

    let cfg = ConfigBuilder::new()
        .artifact_dir(artifact_dir)
        .kernel_binary(kernel_binary)
        .trampoline_binary(trampoline_binary)
        .limine_config(limine_cfg)
        .iso_root(iso_root)
        .iso_name(iso_name)
        .build();

    kbuild::build(&cfg)
}
