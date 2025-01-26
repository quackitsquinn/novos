use std::fs;

use crate::{config::Config, copy_all, macros::cargo_warn};

pub fn update_limine(cfg: &Config) {
    let limine_dir = cfg.a("limine");
    if limine_dir.exists() && !cfg.reinstall_limine {
        return;
    }
    cargo_warn!("Reinstalling limine");
    let _ = fs::remove_dir_all(&limine_dir);
    // Clone limine
    let status = std::process::Command::new("git")
        .args(&[
            "clone",
            "https://github.com/limine-bootloader/limine.git",
            "--branch=v8.x-binary",
            "--depth=1",
            limine_dir.to_str().unwrap(),
        ])
        .spawn()
        .expect("Failed to clone limine")
        .wait()
        .expect("Failed to clone limine");
    assert!(status.success(), "Failed to clone limine!");
    // We do not compile limine here, because we don't need to.
    // All we need is the bootloader binary.
}

pub fn copy_limine_boot(cfg: &Config) {
    let boot = cfg.iso("boot");
    let efi = cfg.iso("EFI/BOOT");
    let limine = cfg.a("limine");

    fs::create_dir_all(&boot)
        .expect(format!("Failed to create iso/boot directory with {:?}", boot).as_str());

    copy_all(&boot, &limine, &vec![
        "limine-bios.sys",
        "limine-bios-cd.bin",
        "limine-uefi-cd.bin",
    ]);

    fs::create_dir_all(&efi).expect("Failed to create iso/EFI/BOOT directory");

    copy_all(&efi, &limine, &vec!["BOOTX64.EFI", "BOOTIA32.EFI"]);

    fs::copy(&cfg.kernel_binary, cfg.iso("boot/kernel.bin")).expect("Failed to copy kernel.bin");

    fs::copy(&cfg.limine_config, cfg.iso("boot/limine.conf")).expect("Failed to copy limine.cfg");
}
