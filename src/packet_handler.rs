use std::{
    fs,
    io::{self, Write},
    os::unix::net::UnixListener,
    panic::catch_unwind,
    path::PathBuf,
    process::Child,
    thread,
};

use kserial::server::SerialHandler;

pub fn run(pty: &PathBuf, qemu: &mut Child) {
    let _ = fs::remove_file(pty);
    let _ = fs::create_dir("output");
    let stdout = JointStdoutFileStream::new();
    for i in 0..10 {
        let listener = match UnixListener::bind(pty) {
            Ok(tty) => tty,
            Err(e) => {
                if i == 9 {
                    panic!("Failed to bind to socket after 10 attempts: {}", e);
                }
                println!("Failed to bind to socket, retrying in 500ms");
                thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }
        };

        let (stream, addr) = listener.accept().expect("Failed to accept connection");
        println!("Connected to {:?}", addr);
        let _ = catch_unwind(|| {
            if let Err(e) = SerialHandler::new(stream)
                .expect("Failed to create stream")
                .with_output(stdout)
                .run()
            {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    println!("Connection closed");
                } else {
                    panic!("SerialHandler ran into an unexpected error: {}", e);
                }
            }
        });
        qemu.kill().expect("Failed to kill QEMU");
        println!("Server stopped");
        break;
    }
}

struct JointStdoutFileStream {
    stdout: fs::File,
}

impl JointStdoutFileStream {
    fn new() -> Self {
        let stdout = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("output/stdout.log")
            .expect("Failed to open stdout log file");
        Self { stdout }
    }
}

impl Write for JointStdoutFileStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdout.write(buf)?;
        io::stdout().write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdout.flush()?;
        io::stdout().flush()
    }
}
