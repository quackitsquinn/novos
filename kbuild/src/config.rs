//! Configuration for building the Nova Kernel.
use std::{env, fmt::Debug, path::PathBuf};

/// Configuration for building and running the Nova Kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path to the kernel binary.
    pub kernel_binary: PathBuf,
    /// Path to the artifact directory.
    pub artifact_dir: PathBuf,
    /// Whether to reinstall Limine bootloader files.
    pub reinstall_limine: bool,
    /// Path to the Limine configuration file.
    pub limine_config: PathBuf,
    /// Path to the ISO root directory.
    pub iso_root: PathBuf,
    /// Name of the output ISO file.
    pub iso_name: String,
}

fn val_or_env<T, E>(val: T, env: &str) -> T
where
    T: std::str::FromStr<Err = E>,
    E: Debug,
{
    match env::var(env) {
        Ok(val) => val.parse().unwrap(),
        Err(_) => val,
    }
}

impl Config {
    /// Create a new `Config`, using environment variables to override any provided values.
    pub fn new(
        artifact_dir: PathBuf,
        kernel_binary: PathBuf,
        limine_config: PathBuf,
        iso_root: PathBuf,
        iso_name: String,
    ) -> Config {
        Config {
            kernel_binary,
            artifact_dir: val_or_env(artifact_dir, "ARTIFACT_DIR"),
            reinstall_limine: val_or_env(false, "REINSTALL_LIMINE"),
            limine_config: val_or_env(limine_config, "LIMINE_CONFIG"),
            iso_root: val_or_env(iso_root, "ISO_ROOT"),
            iso_name: val_or_env(iso_name, "ISO_NAME"),
        }
    }

    /// Joins `path` with the output directory.
    pub fn a(&self, path: &str) -> PathBuf {
        self.artifact_dir.join(path)
    }

    /// Joins `path` with the ISO output directory.
    pub fn iso(&self, path: &str) -> PathBuf {
        self.a(&self.iso_name).join(path)
    }
}
