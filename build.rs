use std::{fs, path::PathBuf};

use kbuild::config::Config;

fn main() {
    let kernel_dir = std::env::var("CARGO_BIN_FILE_KERNEL").expect("CARGO_BIN_FILE_KERNEL not set");
    let dbg_mode = std::env::var("DEBUG").is_ok();

    let limine_cfg = if dbg_mode {
        PathBuf::try_from("boot_cfg/main_debug.conf")
            .expect("Failed to convert boot_cfg/main_debug.conf to PathBuf")
    } else {
        PathBuf::try_from("boot_cfg/main.conf")
            .expect("Failed to convert boot_cfg/main.conf to PathBuf")
    };
    let _ = fs::create_dir("boot_images");
    kbuild::build(&Config::new(
        "target/artifacts"
            .parse()
            .expect("Failed to parse target/artifacts"),
        PathBuf::try_from(kernel_dir).expect("Failed to convert kernel_dir to PathBuf"),
        limine_cfg,
        PathBuf::try_from("boot_images").expect("Failed to convert  to PathBuf"),
        "novaos.iso".to_string(),
    ));
}
