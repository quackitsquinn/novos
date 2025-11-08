use std::{
    collections::HashSet,
    process::Command,
    sync::{Arc, OnceLock},
};

use crate::env::qemu_path;

// We don't create this at runtime because qemu doesn't output the debug flags in a nice to parse way.
/// The standard set of QEMU debug flags that can be used with `-d`.
pub static QEMU_STANDARD_DEBUG_FLAGS: &[&str] = &[
    "out_asm",
    "in_asm",
    "op",
    "op_opt",
    "op_ind",
    "op_plugin",
    "int",
    "exec",
    "cpu",
    "fpu",
    "mmu",
    "pcall",
    "cpu_reset",
    "unimp",
    "guest_errors",
    "non-existent",
    "page",
    "nochain",
    "complete",
    "plugin",
    "strace",
    "vpu",
    "invalid_mem",
];

static QEMU_TRACE_DEBUG_FLAGS: OnceLock<HashSet<Arc<str>>> = OnceLock::new();

/// Returns the list of QEMU trace debug flags that can be used with `-d trace=...`.
pub fn trace_flags() -> &'static HashSet<Arc<str>> {
    fn generate_trace_flags() -> HashSet<Arc<str>> {
        // Count of trace debug flags in QEMU as of version 10.1.2
        let mut flags = HashSet::with_capacity(4840);

        let qemu = Command::new(qemu_path())
            .arg("-d")
            .arg("trace:help")
            .output()
            .expect("Failed to execute QEMU to get trace debug flags");
        let output = String::from_utf8_lossy(&qemu.stdout);

        for line in output.lines() {
            if let Some(flag) = line.split_whitespace().next() {
                flags.insert(Arc::from(flag));
            }
        }

        flags
    }

    QEMU_TRACE_DEBUG_FLAGS.get_or_init(generate_trace_flags)
}

/// Checks if the given debug flag is a standard flag. If this function returns true, the given flag will always be valid.
pub fn flag_is_valid_standard(flag: &str) -> bool {
    // Check standard flags first for efficiency
    QEMU_STANDARD_DEBUG_FLAGS.contains(&flag)
}

/// Checks if the given debug flag is a valid trace flag. If this function returns true,
/// the given flag will always be a valid trace flag.
pub fn flag_is_valid_trace(flag: &str) -> bool {
    trace_flags().contains(flag)
}

/// Checks if the given debug flag is a trace flag. This **doesn't check the validity** of the flag.
/// Returns (bool, String) where the bool indicates if it's a trace flag, and the String is the normalized flag.
/// The String will be the same as the input flag if the bool is false.
pub fn flag_is_trace(flag: &str) -> (bool, String) {
    if flag.starts_with("trace:") {
        return (true, flag[6..].to_string());
    } else if flag.starts_with("t/") || flag.starts_with("t:") {
        return (true, flag[2..].to_string());
    } else {
        return (false, flag.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_is_trace() {
        let cases = vec![
            ("trace:cpu", (true, "cpu".to_string())),
            ("t/cpu", (true, "cpu".to_string())),
            ("t:cpu", (true, "cpu".to_string())),
            ("cpu", (false, "cpu".to_string())),
            ("trace:", (true, "".to_string())),
            ("t/", (true, "".to_string())),
            ("t:", (true, "".to_string())),
        ];
        for (input, expected) in cases {
            let result = flag_is_trace(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_flag_is_valid_standard() {
        for &flag in QEMU_STANDARD_DEBUG_FLAGS {
            assert!(
                flag_is_valid_standard(flag),
                "Flag {} should be valid",
                flag
            );
        }
    }

    #[test]
    fn test_flag_is_valid_trace() {
        let trace_flags = trace_flags();
        for flag in trace_flags.iter() {
            assert!(
                flag_is_valid_trace(flag),
                "Trace flag {} should be valid",
                flag
            );
        }
    }
}
