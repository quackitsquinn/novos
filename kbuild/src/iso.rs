use std::{panic, process::Command};

use crate::config::Config;

pub fn make_iso(cfg: &Config) {
    if !Command::new("which")
        .arg("xorriso")
        .spawn()
        .expect("Failed to find xorriso")
        .wait()
        .expect("Failed to find xorriso")
        .success()
    {
        panic!("xorriso not found");
    }

    let iso_dir = cfg.iso("");
    let out = cfg.iso_root.join(&cfg.iso_name);

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
        &iso_dir.to_str().unwrap(),
        "-o",
        &out.to_str().unwrap(),
    ]);

    let status = iso
        .spawn()
        .expect("Failed to spawn xorriso")
        .wait()
        .expect("Failed to wait for xorriso");

    assert!(status.success(), "Failed to create iso!");
}
