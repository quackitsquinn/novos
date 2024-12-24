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

    if invalidate_limine {
        println!("cargo:warning=Invalidating limine");
    }

    println!("OUT_DIR: {}", out_dir);
    println!("CARGO_BIN_FILE_KERNEL: {}", kernel_dir);

    make_limine_bin(invalidate_limine);

    make_iso("novos.iso", &kernel_dir, "boot_cfg/main.conf");
    // debug disables KASLR which breaks lldb's ability to find the symbols
    make_iso("novos_debug.iso", &kernel_dir, "boot_cfg/debug.conf");

    build_tests();

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
    std::process::Command::new("make")
        .arg("limine")
        .current_dir(out_base!("limine/"))
        .spawn()
        .expect("Failed to compile limine")
        .wait()
        .expect("Failed to compile limine");
}

fn rm_rf(path: &str) {
    let _ = fs::remove_dir_all(path);
}

fn make_iso(out_name: &str, kernel_bin: &str, config: &str) {
    fs::remove_dir_all(out_base!("iso")).ok();
    fs::create_dir_all(out_base!("iso/boot")).expect("Failed to create iso/boot directory");

    copy_all!(
        out_base!("iso/boot/"),
        &limine!("limine-bios.sys"),
        &limine!("limine-bios-cd.bin"),
        &limine!("limine-uefi-cd.bin")
    );
    fs::copy(kernel_bin, out_base!("iso/boot/kernel.bin")).expect("Failed to copy kernel.bin");
    fs::copy(config, out_base!("iso/boot/limine.conf")).expect("Failed to copy limine.cfg");

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
        &out_base!(out_name),
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

fn build_tests() {
    // TODO: Refactor to use cargo build --tests and with json outputs
    //
    // Build kernel tests
    println!("Building kernel tests");
    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .current_dir("kernel")
        .args(&["test", "--no-run"])
        .env(
            "CARGO_TARGET_DIR",
            fs::canonicalize("target/tests").unwrap_or_else(|f| {
                if f.kind() == std::io::ErrorKind::NotFound {
                    fs::create_dir("target/tests").expect("Failed to create target/tests");
                    return fs::canonicalize("target/tests").expect("Failed to get canonical path");
                }
                panic!("Failed to get canonical path for target/tests: {}", f)
            }),
        )
        .output()
        .unwrap_or_else(|f| {
            println!("cargo::warning=Failed to run cargo test --no-run: {}", f);
            panic!("Failed to run cargo test --no-run: {}", f)
        });

    // The last line should contain the full path to the test binary.
    // This whole thing is kinda gross, but as far as I can tell, there is no way to get the path to the built binary from cargo
    let output_str = String::from_utf8_lossy(&output.stderr); // Cargo put most of it's output in stderr
    println!("Output from cargo test --no-run: {}", output_str);
    let output = output_str.lines().last().unwrap();
    let bin = output.split("(").nth(1).unwrap().trim_end_matches(")");
    println!("Test binary: {}", bin);
    make_iso("kernel_tests.iso", bin, "boot_cfg/debug.conf");
    // Copy the test binary to the artifacts directory
    fs::copy(bin, out_base!("kernel_tests.bin")).expect("Failed to copy kernel_tests");
}

fn make_hdd(out_dir: &str, kernel_bin_dir: &str) {}
