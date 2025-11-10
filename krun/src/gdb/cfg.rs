use std::{env::VarError, fs, io::ErrorKind};

use toml::{Value, map::Map};

use super::invocation::GdbInvocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GdbConfig {
    pub invocation: Option<GdbInvocation>,
    pub port: u16,
    pub host: String,
}

const GDB_INVOCATION: &str = "GDB_INVOCATION";
const GDB_PORT: &str = "GDB_PORT";
const GDB_HOST: &str = "GDB_HOST";
const DEFAULT_GDB_TOML: &str = r#"
# Default GDB configuration

[connection]
host = "localhost"
port = 1234
invocation = "gdb"


"#;

impl Default for GdbConfig {
    fn default() -> Self {
        let mut default = GdbConfig {
            invocation: Some(GdbInvocation::empty()),
            port: 1234,
            host: "localhost".to_string(),
        };
        default.apply_cfg();
        default.apply_env();
        println!("Using GDB configuration: {:?}", default);
        default
    }
}

impl GdbConfig {
    fn try_apply(
        &mut self,
        invocation: Option<String>,
        port: Option<u16>,
        host: Option<String>,
    ) -> Option<()> {
        if let Some(path) = invocation {
            let path = GdbInvocation::from_invocation_string(&path);
            if path.is_none() {
                eprintln!("Invalid gdb path value");
                return None;
            }
            self.invocation = path;
        }
        if let Some(p) = port {
            self.port = p;
        }
        if let Some(h) = host {
            self.host = h;
        }
        Some(())
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

        let gdb_invocation = get_var(GDB_INVOCATION);
        let port = get_var(GDB_PORT).map(|p| p.parse::<u16>().expect("Invalid GDB_PORT value"));
        let host = get_var(GDB_HOST);

        if self.try_apply(gdb_invocation, port, host).is_none() {
            eprintln!("Failed to apply environment variables");
            return;
        }
    }

    pub fn apply_cfg(&mut self) {
        let cfg = fs::read_to_string("gdb.toml").map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                eprintln!("gdb.toml not found, writing default configuration");
                write_default_gdb_toml();
            } else {
                eprintln!("Error reading gdb.toml: {}", e);
            }
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
            .as_table()
            .expect("Invalid 'connection' section in gdb.toml");

        fn get_key<T>(
            cfg: &Map<String, Value>,
            key: &str,
            convert: fn(&Value) -> Option<T>,
        ) -> Option<T> {
            let val = cfg.get(key)?;
            let converted = convert(val);
            if converted.is_none() {
                eprintln!("Invalid value for {} in gdb.toml", key);
            }
            converted
        }

        let gdb_path = get_key(cfg, "invocation", |v| v.as_str().map(|s| s.to_string()));
        let gdb_port = get_key(cfg, "port", |v| v.as_integer().map(|i| i as u16));
        let gdb_host = get_key(cfg, "host", |v| v.as_str().map(|s| s.to_string()));

        println!(
            "Applying GDB configuration from gdb.toml: {:?}, {:?}, {:?}",
            gdb_path, gdb_port, gdb_host
        );

        if self.try_apply(gdb_path, gdb_port, gdb_host).is_none() {
            eprintln!("Failed to apply gdb.toml configuration");
            return;
        }
    }
}

fn write_default_gdb_toml() {
    let default_cfg = DEFAULT_GDB_TOML;
    fs::write("gdb.toml", default_cfg)
        .map(|_| {
            eprintln!("Default gdb.toml written to current directory");
        })
        .unwrap_or_else(|e| {
            eprintln!("Failed to write default gdb.toml: {}", e);
        });
}
