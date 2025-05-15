use std::{env::VarError, fs, option, path::PathBuf};

use which::which;

pub struct GdbInstance {
    gdb: std::process::Child,
}

pub struct GdbConfig {
    pub invocation: GdbInvocation,
    pub port: u16,
    pub host: String,
}

const GDB_NOT_FOUND: &str = "GDB not found in PATH! Either specify the path to gdb in the config or the GDB_PATH environment variable.";

impl Default for GdbConfig {
    fn default() -> Self {
        GdbConfig {
            invocation: GdbInvocation::default(),
            port: 1234,
            host: "localhost".to_string(),
        }
    }
}

impl GdbConfig {
    pub fn new(invocation: GdbInvocation, port: u16, host: String) -> Self {
        GdbConfig {
            invocation,
            port,
            host,
        }
    }

    pub fn apply(
        &mut self,
        invocation: Option<GdbInvocation>,
        port: Option<u16>,
        host: Option<String>,
    ) {
        if let Some(path) = invocation {
            self.invocation = path;
        }
        if let Some(p) = port {
            self.port = p;
        }
        if let Some(h) = host {
            self.host = h;
        }
    }

    pub fn apply_env(&mut self) {
        let get_var = |var: &str| -> Option<String> {
            match std::env::var(var) {
                Ok(val) => Some(val),
                Err(VarError::NotPresent) => None,
                Err(VarError::NotUnicode(_)) => {
                    panic!("Environment variable {} is not valid UTF-8", var)
                }
            }
        };

        let gdb_invocation = get_var("GDB_PATH").map(|p| {});
        let port = get_var("GDB_PORT").map(|p| p.parse::<u16>().expect("Invalid GDB_PORT value"));
        let host = get_var("GDB_HOST");
    }

    pub fn apply_cfg(&mut self) {
        let cfg = fs::read_to_string("gdb.toml").map_err(|e| {
            eprintln!("Failed to read gdb.toml: {}", e);
            e
        });
        if cfg.is_err() {
            eprintln!("Using default GDB configuration");
            return;
        }
        let cfg: toml::Value = toml::from_str(&cfg.unwrap()).expect("Failed to parse gdb.toml");
        let cfg = cfg
            .get("connection")
            .expect("Missing 'connection' section in gdb.toml")
            .as_table();
        if let Some(invocation) = cfg.get("invocation") {
            if let Some(invocation_str) = invocation.as_str() {
                if let Some(inv) = GdbInvocation::from_invocation_string(invocation_str) {
                    self.invocation = inv;
                } else {
                    eprintln!("Invalid GDB invocation string: {}", invocation_str);
                }
            }
        }
    }
}

pub struct GdbInvocation {
    args: Vec<String>,
    gdb_path: PathBuf,
    insert_point: usize,
}

impl GdbInvocation {
    pub fn from_invocation_string(invocation: &str) -> Option<Self> {
        let args: Vec<String> = invocation
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let insert_point = args.iter().position(|s| s == "{}").unwrap_or(args.len());
        let gdb_path = eprint_option(
            args.first().cloned(),
            "Invocation string empty or missing GDB path",
        )?;
        let gdb_path = which(gdb_path);
        if gdb_path.is_err() {
            eprintln!(
                "GDB not found at specified path or in PATH: {}",
                gdb_path.unwrap_err()
            );
            return None;
        }
        let gdb_path = gdb_path.unwrap();
        Some(GdbInvocation {
            args,
            gdb_path,
            insert_point,
        })
    }

    pub fn invoke(&self, args: Vec<String>) -> std::process::Child {
        let mut command = std::process::Command::new(&self.gdb_path);
        command.args(self.args.clone());
        if self.insert_point < self.args.len() {
            command.args(&args[..self.insert_point]);
        }
        command.args(args);
        if self.insert_point < self.args.len() {
            command.args(&self.args[self.insert_point + 1..]);
        }
        command.spawn().expect("Failed to start GDB")
    }
}

impl Default for GdbInvocation {
    fn default() -> Self {
        GdbInvocation {
            args: vec![],
            gdb_path: which("gdb").expect(GDB_NOT_FOUND),
            insert_point: 0,
        }
    }
}

pub(crate) fn eprint_option<T>(option: Option<T>, msg: &str) -> Option<T> {
    if option.is_none() {
        eprintln!("{}", msg);
    }
    option
}
