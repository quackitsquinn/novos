//!! Kernel build utilities: ISO creation and bootloader setup.
use std::{fs, path::Path};

use config::Config;

pub mod config;
mod iso;
mod limine;

/// Builds the kernel ISO and sets up the Limine bootloader.
pub fn build(cfg: &Config) {
    fs::create_dir_all(&cfg.a("iso")).ok();
    limine::update_limine(&cfg);
    limine::copy_limine_boot(&cfg);
    iso::make_iso(&cfg);
}

mod macros {
    macro_rules! cargo_warn {
        ($r: tt) => {
            println!("cargo:warning={}", $r);
        };
    }

    pub(crate) use cargo_warn;
}

pub(crate) fn copy_all(dst: &Path, src_base: &Path, src: &[&str]) {
    for s in src {
        let path = src_base.join(s);
        let fname = path.file_name();
        let dst = dst.join(fname.expect("Unable to get file name"));
        println!("Copying {} to {:?}", s, dst);
        std::fs::copy(path, &dst).expect(&format!("failed to copy {} to {:?} ", s, dst));
    }
}
