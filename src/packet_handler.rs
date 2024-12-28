use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    thread,
};

use kserial::server::Server;

pub fn run(pty: &PathBuf) {
    // Just print the raw output to the console
    let stream = for i in 0..10 {
        let mut tty = match UnixStream::connect(pty) {
            Ok(tty) => tty,
            Err(e) => {
                if i == 9 {
                    panic!("Failed to connect to tty: {}", e);
                }
                thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        };

        let qemu_out = File::create("qemu_out.txt").expect("Failed to create qemu_out.txt");
        let mut qemu_out = std::io::BufWriter::new(qemu_out);
        let mut buf = [0; 64];
        loop {
            let len = tty.read(&mut buf).expect("Failed to read from tty");
            qemu_out
                .write_all(&buf[..len])
                .expect("Failed to write to qemu_out.txt");
            qemu_out.flush().expect("Failed to flush qemu_out.txt");
        }
        //Server::new(&pty).unwrap().run().unwrap();
    };
}
