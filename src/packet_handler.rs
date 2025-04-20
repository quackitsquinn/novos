use std::{
    fs, os::unix::net::UnixListener, panic::catch_unwind, path::PathBuf, process::Child, thread,
};

use kserial::server::SerialHandler;

pub fn run(pty: &PathBuf, qemu: &mut Child) {
    let _ = fs::remove_file(pty);
    let _ = fs::create_dir("output");
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
            if let Err(e) = SerialHandler::new(stream).unwrap().run() {
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
