use std::{env, fmt::Debug, path::PathBuf};

pub struct Config {
    pub kernel_binary: PathBuf,
    pub artifact_dir: PathBuf,
    pub reinstall_limine: bool,
    pub limine_config: PathBuf,
    pub iso_root: PathBuf,
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

    pub fn iso(&self, path: &str) -> PathBuf {
        self.a(&self.iso_name).join(path)
    }
}
