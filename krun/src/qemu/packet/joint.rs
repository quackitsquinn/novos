//! haha weed haha 420

use std::{
    fs,
    io::{self, Write},
    panic::UnwindSafe,
    path::Path,
};

/// A writer that writes to both a file and stdout, converting `\n` to `\n\r` for stdout.
pub struct JointStdoutFileStream {
    stdout: fs::File,
}

impl JointStdoutFileStream {
    /// Creates a new `JointStdoutFileStream` that writes to the specified file path.
    pub fn new(path: &Path) -> io::Result<Self> {
        let stdout = fs::OpenOptions::new().write(true).create(true).open(path)?;
        Ok(Self { stdout })
    }
}

impl Write for JointStdoutFileStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdout.write(buf)?;
        let mut stdout = io::stdout();
        for byte in buf {
            if *byte == b'\n' {
                // The \r fixes weird terminal behavior in some situations
                stdout.write_all(b"\n\r")?;
            } else {
                stdout.write_all(&[*byte])?;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdout.flush()?;
        io::stdout().flush()
    }
}

impl UnwindSafe for JointStdoutFileStream {}
