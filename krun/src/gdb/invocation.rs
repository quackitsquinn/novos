use std::path::PathBuf;
use which::which;

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
            gdb_path: which("gdb").expect("GDB not found in PATH! Either specify the path to gdb in the config or the GDB_PATH environment variable."),
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
