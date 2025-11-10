use std::{
    fmt, fs,
    io::{self, ErrorKind, stdout},
    os::unix::{fs::FileTypeExt, net::UnixListener},
    panic::{AssertUnwindSafe, UnwindSafe, catch_unwind},
    path::Path,
    thread,
    time::Duration,
};

mod joint;

use kserial::server::SerialHandler;

use crate::qemu::{controller::QemuCtl, packet::joint::JointStdoutFileStream};

/// Loads the packet mode for QEMU communication over the provided socket.
pub fn load_packet_mode<T>(rw: T)
where
    T: io::Read + io::Write + Send + 'static + UnwindSafe,
{
    match fs::create_dir("output") {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        Err(e) => panic!("Failed to create output directory: {}", e),
    }

    let stdout = JointStdoutFileStream::new(&Path::new("output/kserial.log"))
        .expect("Failed to create JointStdoutFileStream");

    let handler = SerialHandler::new(rw)
        .expect("Failed to create stream")
        .with_output(stdout);

    let handler = AssertUnwindSafe(handler);

    let err = match handler.0.run() {
        Ok(_) => {
            println!("Connection closed");
            return;
        }
        Err(e) => e,
    };

    if err.kind() == std::io::ErrorKind::UnexpectedEof {
        println!("Connection closed");
    } else {
        panic!("SerialHandler ran into an unexpected error: {}", err);
    }
}

pub const SOCKET_CREATION_WAIT_INTERVAL: Duration = Duration::from_millis(500);
pub const MAX_SOCKET_CREATION_ATTEMPTS: u8 = 10;

/// Creates a Unix socket listener at the specified path, handling existing sockets appropriately.
pub fn create_unix_socket_listener(path: &std::path::Path) -> io::Result<UnixListener> {
    for i in 0..MAX_SOCKET_CREATION_ATTEMPTS {
        match UnixListener::bind(path) {
            Ok(tty) => return Ok(tty),
            Err(e) if e.kind() == ErrorKind::AddrInUse => {
                handle_remove_old_socket(path)?;
            }
            Err(e) => {
                if i == MAX_SOCKET_CREATION_ATTEMPTS - 1 {
                    return Err(e);
                }
                println!("Failed to bind to socket, retrying in 500ms");
                thread::sleep(SOCKET_CREATION_WAIT_INTERVAL);
            }
        }
    }
    unreachable!()
}

fn handle_remove_old_socket(path: &std::path::Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = std::fs::metadata(path)?;
    if metadata.file_type().is_socket() {
        fs::remove_file(path)?;
    } else {
        return Err(io::Error::new(
            ErrorKind::Other,
            format!("Path {} exists and is not a socket", path.display()),
        ));
    }
    Ok(())
}
