use std::{fs, os::unix::net::UnixListener, path::PathBuf, thread};

use kserial::server::SerialHandler;

pub fn run(pty: &PathBuf) {
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

        SerialHandler::new(stream).unwrap().run().unwrap();
        println!("Server stopped");
        break;
    }
}
