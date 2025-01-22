use std::{fs, path::PathBuf};

use kbuild::config::Config;

fn main() {
    let kernel_dir = std::env::var("CARGO_BIN_FILE_KERNEL").expect("CARGO_BIN_FILE_KERNEL not set");
    let _ = fs::create_dir("boot_images");
    kbuild::build(&Config::new(
        "target/artifacts"
            .parse()
            .expect("Failed to parse target/artifacts"),
        PathBuf::try_from(kernel_dir).expect("Failed to convert kernel_dir to PathBuf"),
        PathBuf::try_from("boot_cfg/main.conf")
            .expect("Failed to convert boot_cfg/main.conf to PathBuf"),
        PathBuf::try_from("boot_images").expect("Failed to convert  to PathBuf"),
        "novaos.iso".to_string(),
    ));
}
