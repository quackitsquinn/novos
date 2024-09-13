use std::{fs, path::Path, process::Command};

const OUT_BASE: &str = "target/artifacts";

macro_rules! out_base {
    ($path:expr) => {
        format!("{}/{}", OUT_BASE, $path)
    };
}

macro_rules! limine {
    ($path:expr) => {
        format!("{}/{}", out_base!("limine"), $path)
    };
}
macro_rules! copy_all {
    ($dst:expr, $($src:expr),*) => {
        $(
            let as_str = $src;
            let path = Path::new(as_str);
            let fname = path.file_name();
            let file_name = fname.expect("Unable to get file name").to_str();

            let dst = format!("{}{}", $dst, file_name.unwrap());
            println!("Copying {} to {}", $src, dst);
            fs::copy($src,&dst).expect(&format!("failed to copy {} to {} ", $src, dst));
    )*
    };
}

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let kernel_dir = std::env::var("CARGO_BIN_FILE_KERNEL").expect("CARGO_BIN_FILE_KERNEL not set");
    // Should re-clone limine if the env var is set
    let invalidate_limine = std::env::var("INVALIDATE_LIMINE").is_ok();

    println!("OUT_DIR: {}", out_dir);
    println!("CARGO_BIN_FILE_KERNEL: {}", kernel_dir);

    make_limine_bin(invalidate_limine);

    make_iso(&out_dir, &kernel_dir);

    copy_kernel_bin_dbg(&out_dir, &kernel_dir);

    make_hdd(&out_dir, &kernel_dir);
}

/// Build limine binary
fn make_limine_bin(invalidate: bool) {
    if Path::new(&out_base!("limine/")).exists() && !invalidate {
        return;
    } else {
        rm_rf(&out_base!("limine/"));
        // Clone limine
        let output = std::process::Command::new("git")
            .args(&[
                "clone",
                "https://github.com/limine-bootloader/limine.git",
                "--branch=v8.x-binary",
                "--depth=1",
                &out_base!("limine"),
            ])
            .output()
            .expect("Failed to clone limine");
        println!("status: {}", output.status);
    }
    // Compile limine
    let output = std::process::Command::new("make")
        .arg("limine")
        .current_dir(out_base!("limine/"))
        .output()
        .expect("Failed to compile limine");
}

fn rm_rf(path: &str) {
    let _ = fs::remove_dir_all(path);
}

fn make_iso(out_dir: &str, kernel_bin: &str) {
    fs::create_dir_all(out_base!("iso/boot")).expect("Failed to create iso/boot directory");

    copy_all!(
        out_base!("iso/boot/"),
        "limine.conf",
        &limine!("limine-bios.sys"),
        &limine!("limine-bios-cd.bin"),
        &limine!("limine-uefi-cd.bin")
    );
    fs::copy(kernel_bin, out_base!("iso/boot/kernel.bin")).expect("Failed to copy kernel.bin");

    fs::create_dir_all(out_base!("iso/EFI/BOOT")).expect("Failed to create iso/EFI/BOOT directory");

    copy_all!(
        out_base!("iso/EFI/BOOT/"),
        &limine!("BOOTX64.EFI"),
        &limine!("BOOTIA32.EFI")
    );
    let mut iso = Command::new("xorriso");
    // This is roughly taken from the limine example. Just in rust rather than bash
    let iso = iso.args(&[
        "-as",
        "mkisofs",
        "-b",
        "boot/limine-bios-cd.bin",
        "--no-emul-boot",
        "--boot-load-size",
        "4",
        "--boot-info-table",
        "--efi-boot",
        "boot/limine-uefi-cd.bin",
        "--efi-boot-part",
        "--efi-boot-image",
        "--protective-msdos-label",
        &out_base!("iso"),
        "-o",
        &out_base!("novos.iso"),
    ]);
    println!("{:?}", format!("{:?}", iso).replace("\"", ""));
    let output = iso.output();
    if let Ok(output) = output {
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("Failed to create iso: {:?}", output);
    }
}

fn copy_kernel_bin_dbg(out_dir: &str, kernel_bin_dir: &str) {
    // Check if we are in release mode
    let release = std::env::var("PROFILE").unwrap() == "release";
    // We only copy the binary if we are in debug mode because we need to copy the debug symbols
    if !release {
        fs::copy(kernel_bin_dir, out_base!("kernel.bin")).expect("Failed to copy kernel.bin");
    }
}

fn make_hdd(out_dir: &str, kernel_bin_dir: &str) {}
