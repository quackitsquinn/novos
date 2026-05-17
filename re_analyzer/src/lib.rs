//! A library for analyzing Rust metadata and generating a resolved rust-project.json dependency graph.
use std::{io, sync::Arc};

use escargot::Cargo;

pub mod meta;
pub mod resolve;

/// Runs `cargo metadata` and returns the output as a string.
pub fn run_metadata() -> Result<Arc<str>, io::Error> {
    let cargo = Cargo::new().args(&["metadata", "--format-version=1"]);
    let output = cargo.into_command().output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    return Ok(Arc::from(
        String::from_utf8_lossy(&output.stdout).to_string(),
    ));
}
