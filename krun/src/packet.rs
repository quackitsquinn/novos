use std::{
    fs,
    io::{self, Write},
    os::unix::net::UnixListener,
    panic::catch_unwind,
    thread,
    time::Duration,
};

use kserial::server::SerialHandler;

use crate::qemu_ctl::QemuCtl;

const SOCKET_CREATION_WAIT_INTERVAL: Duration = Duration::from_millis(500);
const MAX_SOCKET_CREATION_ATTEMPTS: u8 = 10;

pub fn run_kserial(qemu: QemuCtl) {
    let pty = qemu.get_pty_path();
    let _ = fs::remove_file(&pty);
    let _ = fs::create_dir("output");
    let stdout = JointStdoutFileStream::new();
    for i in 0..MAX_SOCKET_CREATION_ATTEMPTS {
        let listener = match UnixListener::bind(&pty) {
            Ok(tty) => tty,
            Err(e) => {
                if i == 9 {
                    panic!("Failed to bind to socket after 10 attempts: {}", e);
                }
                println!("Failed to bind to socket, retrying in 500ms");
                thread::sleep(SOCKET_CREATION_WAIT_INTERVAL);
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
        qemu.try_shutdown().expect("Failed to shutdown QEMU");
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
        let mut stdout = io::stdout();
        for byte in buf {
            if *byte == b'\n' {
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
