use std::{
    env::VarError,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::qemu::debug::{flag_is_trace, flag_is_valid_standard, flag_is_valid_trace};

/// Environment variable to control debugger attachment.
/// If set to "1", "true", or "yes", QEMU will wait for GDB to attach.
/// If set to "nowait", "no-wait", or "no_wait", QEMU will start a GDB server but not wait for a debugger to attach.
pub const DEBUGGER_ENV_FLAG: &str = "DEBUG";
/// Environment variable to enable UEFI booting. If set to any value, UEFI will be enabled.
pub const UEFI_ENV_FLAG: &str = "UEFI";
/// Environment variable to disable QEMU graphical output. If set to any value, QEMU will run without a display.
pub const NO_GRAPHIC_ENV_FLAG: &str = "NO_DISPLAY";
/// Environment variable to specify the kernel image path. This should be a path to an ISO file.
pub const IMAGE_PATH_ENV_FLAG: &str = "KERNEL_IMAGE_PATH";
/// Environment variable to specify the amount of memory for the kernel. This can also include hotpluggable memory settings.
pub const MEM_ENV_FLAG: &str = "QEMU_MEM";
/// Environment variable to enable QEMU's dev exit feature. If set to any value, dev exit will be enabled.
pub const DEV_EXIT_ENV_FLAG: &str = "DEV_EXIT";
/// Environment variable to specify extra QEMU arguments.
pub const EXTRA_ARGS_ENV_FLAG: &str = "QEMU_ARGS";
/// Environment variable to enable verbose mode.
pub const VERBOSE_ENV_FLAG: &str = "VERBOSE";
/// Environment variable to prevent spawning GDB automatically.
pub const NO_SPAWN_GDB_ENV_FLAG: &str = "NO_SPAWN_GDB";
/// Environment variable to specify a custom QEMU binary path.
pub const QEMU_BINARY_ENV_FLAG: &str = "QEMU_PATH";
/// Environment variable to specify the number of SMP cores.
pub const SMP_CORES_ENV_FLAG: &str = "SMP_CORES";
/// Environment variable to specify additional QEMU debug flags. Flags should be comma-separated.
///
/// You can list valid flags by running `qemu-system-x86_64 -d help`.
///
/// You can list trace flags by running`qemu-system-x86_64 -d trace:help`.
/// Warning: there are almost 5000 trace flags.
///
/// Trace flags must be prefixed with `trace:` but the following shorthands are supported:
/// - `t/` -> `trace:`
/// - `trace/` -> `trace:`
/// - `t:` -> `trace:`
pub const QEMU_DEBUG_FLAGS_ENV_FLAG: &str = "QEMU_DEBUG_OPTS";

fn read_env(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(val) => Some(val),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => {
            panic!("Environment variable {} is not valid unicode", key);
        }
    }
}

fn env_present(key: &str) -> bool {
    // Don't care about the value, just if it's set
    match std::env::var(key) {
        Ok(_) | Err(VarError::NotUnicode(_)) => true,
        Err(VarError::NotPresent) => false,
    }
}

/// Returns if qemu should be configured to (start a gdb server, wait for gdb).
pub fn should_attach_debugger() -> (bool, bool) {
    let debug = read_env(DEBUGGER_ENV_FLAG);

    if debug.is_none() {
        return (false, false);
    }

    let mut debug = debug.unwrap();
    debug.make_ascii_lowercase();

    match debug.as_str() {
        "" | "1" | "true" | "yes" => (true, true),
        "nowait" | "no-wait" | "no_wait" => (true, false),
        _ => {
            eprintln!("Unrecognized value for {}: {}", DEBUGGER_ENV_FLAG, debug);
            (false, false)
        }
    }
}

/// Returns if UEFI is enabled.
pub fn uefi_enabled() -> bool {
    env_present(UEFI_ENV_FLAG)
}

/// Returns if display is enabled.
pub fn display_enabled() -> bool {
    !env_present(NO_GRAPHIC_ENV_FLAG)
}

/// Default kernel image path if none is specified.
pub const DEFAULT_KERNEL_IMAGE_PATH: &str = "boot_images/novaos.iso";

/// Returns the kernel image path.
pub fn kernel_image_path() -> PathBuf {
    let path = read_env(IMAGE_PATH_ENV_FLAG);

    match path {
        Some(p) => {
            let image = PathBuf::from(p);
            if !image.exists() {
                panic!(
                    "Kernel image path specified in {} does not exist: {}",
                    IMAGE_PATH_ENV_FLAG,
                    image.display()
                );
            }
            image
        }
        None => PathBuf::from(DEFAULT_KERNEL_IMAGE_PATH),
    }
}

/// Default memory size for QEMU if none is specified.
pub const DEFAULT_QEMU_MEMORY: &str = "1G";

/// Returns the configured memory settings for QEMU. This can include features like hotplug memory and does not only
/// represent a single memory size.
///
/// Defaults to "1G" if the environment variable is not set.
pub fn memory_config() -> String {
    match read_env(MEM_ENV_FLAG) {
        Some(size) => size,
        None => DEFAULT_QEMU_MEMORY.to_string(),
    }
}

/// Returns if QEMU's dev exit feature is enabled.
pub fn dev_exit_enabled() -> bool {
    env_present(DEV_EXIT_ENV_FLAG)
}

/// Returns extra QEMU arguments specified in the environment variable.
pub fn extra_arguments() -> Vec<String> {
    let extra_args = read_env(EXTRA_ARGS_ENV_FLAG);
    match extra_args {
        Some(args) => args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
        None => vec![],
    }
}

/// Returns if verbose mode is enabled.
pub fn verbose_mode() -> bool {
    env_present(VERBOSE_ENV_FLAG)
}

/// Returns if GDB should be spawned automatically.
pub fn should_spawn_gdb() -> bool {
    !env_present(NO_SPAWN_GDB_ENV_FLAG)
}

/// Default QEMU binary to use if none is specified.
pub const DEFAULT_QEMU: &str = "qemu-system-x86_64";

/// Returns the path to the QEMU binary to use.
pub fn qemu_path() -> &'static Path {
    static QEMU: OnceLock<PathBuf> = OnceLock::new();
    QEMU.get_or_init(|| which::which(read_env(QEMU_BINARY_ENV_FLAG).unwrap_or(DEFAULT_QEMU.to_string())).expect(
        "Unable to find qemu-system-x86_64 in PATH! 
             Please ensure QEMU is in PATH or that QEMU_PATH points to a valid qemu-system-x86_64 binary!",
    ))
}

/// Returns the number of SMP cores to use, if specified.
pub fn smp_cores() -> Option<usize> {
    let cores_str = read_env(SMP_CORES_ENV_FLAG)?;
    match cores_str.parse::<usize>() {
        Ok(cores) if cores > 0 => Some(cores),
        _ => {
            panic!(
                "Invalid value for {}: {}. Must be a positive integer.",
                SMP_CORES_ENV_FLAG, cores_str
            );
        }
    }
}

/// Returns the list of QEMU debug flags to use, based on the environment variable.
pub fn qemu_debug_flags() -> Option<Vec<String>> {
    let debug_opts = read_env(QEMU_DEBUG_FLAGS_ENV_FLAG)?;
    let mut flags = Vec::new();

    let check_flag_valid = |flag: &str, is_trace: bool| {
        if is_trace {
            if !flag_is_valid_trace(flag) {
                panic!(
                    "Invalid QEMU trace debug flag specified in {}: {}",
                    QEMU_DEBUG_FLAGS_ENV_FLAG, flag
                );
            }
        } else {
            if !flag_is_valid_standard(flag) {
                panic!(
                    "Invalid QEMU standard debug flag specified in {}: {}",
                    QEMU_DEBUG_FLAGS_ENV_FLAG, flag
                );
            }
        }
    };

    for flag in debug_opts.split(',') {
        let flag = flag.trim();
        if flag.is_empty() {
            continue;
        }

        let (is_trace, flag) = flag_is_trace(flag);

        check_flag_valid(&flag, is_trace);

        if is_trace {
            flags.push(format!("trace:{}", flag));
        } else {
            flags.push(flag);
        }
    }

    if flags.is_empty() {
        return None;
    }
    Some(flags)
}
