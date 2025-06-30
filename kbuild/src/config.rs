use std::{env, fmt::Debug, path::PathBuf};

pub struct Config {
    pub trampoline_binary: PathBuf,
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
    /// Joins `path` with the output directory.
    pub fn a(&self, path: &str) -> PathBuf {
        self.artifact_dir.join(path)
    }

    pub fn iso(&self, path: &str) -> PathBuf {
        self.a(&self.iso_name).join(path)
    }
}

pub struct ConfigBuilder {
    artifact_dir: Option<PathBuf>,
    trampoline_binary: Option<PathBuf>,
    kernel_binary: Option<PathBuf>,
    reinstall_limine: bool,
    limine_config: Option<PathBuf>,
    iso_root: Option<PathBuf>,
    iso_name: Option<String>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            artifact_dir: None,
            trampoline_binary: None,
            kernel_binary: None,
            reinstall_limine: false,
            limine_config: None,
            iso_root: None,
            iso_name: None,
        }
    }

    pub fn reinstall_limine(mut self) -> Self {
        self.reinstall_limine = true;
        self
    }

    pub fn artifact_dir(mut self, artifact_dir: PathBuf) -> Self {
        self.artifact_dir = Some(artifact_dir);
        self
    }

    pub fn kernel_binary(mut self, kernel_binary: PathBuf) -> Self {
        self.kernel_binary = Some(kernel_binary);
        self
    }

    pub fn limine_config(mut self, limine_config: PathBuf) -> Self {
        self.limine_config = Some(limine_config);
        self
    }

    pub fn iso_root(mut self, iso_root: PathBuf) -> Self {
        self.iso_root = Some(iso_root);
        self
    }

    pub fn iso_name(mut self, iso_name: String) -> Self {
        self.iso_name = Some(iso_name);
        self
    }

    pub fn trampoline_binary(mut self, trampoline_binary: PathBuf) -> Self {
        self.trampoline_binary = Some(trampoline_binary);
        self
    }

    pub fn build(self) -> Config {
        Config {
            kernel_binary: self.kernel_binary.expect("kernel_binary is required"),
            trampoline_binary: self
                .trampoline_binary
                .expect("trampoline_binary is required"),
            artifact_dir: self
                .artifact_dir
                .unwrap_or_else(|| PathBuf::from("target/artifacts")),
            reinstall_limine: self.reinstall_limine,
            limine_config: self
                .limine_config
                .unwrap_or_else(|| PathBuf::from("boot_cfg/main.conf")),
            iso_root: self
                .iso_root
                .unwrap_or_else(|| PathBuf::from("boot_images")),
            iso_name: self.iso_name.unwrap_or_else(|| "novaos.iso".to_string()),
        }
    }
}
