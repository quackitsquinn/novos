use core::time::Duration;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::unix::net::UnixStream,
    path::Path,
    thread,
};

use serialport::{SerialPort, SerialPortBuilder, TTYPort};

const BUF_SIZE: usize = 16;

pub struct Server {
    tty: UnixStream,
}

impl Server {
    /// Creates a new server with the given path. The path should be a path to a Unix socket.
    pub fn new(path: &Path) -> Result<Self, io::Error> {
        todo!();
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        // First off, we read all bytes from the tty until we encounter a 0xFF byte. This is the handshake byte, so once we read it, we switch to packet mode.
        read_until_ff(&mut self.tty)?;
        println!("Got 0xFF byte, switching to packet mode");
        // dude idfk spam 0xFF bytes. The qemu serial port is weird, and like, incredibly unreliable.
        // I have tried so many things to do this properly, including using a second port to read commands but LITERALLY NOTHING WORKS

        for _ in 0..10 {
            if let Ok(n) = self.tty.write(&[0xFF]) {
                if n != 1 {
                    println!("Failed to write 0xFF byte");
                }
            }
            //thread::sleep(Duration::from_millis(10));
        }
        self.tty.flush()?;
        let mut buf = [0; BUF_SIZE];
        loop {
            self.tty.read_exact(&mut buf)?;
            let text = String::from_utf8_lossy(&buf);
            print!("{}", text);
        }
    }
}
/// Reads all bytes until it encounters a 0xFF byte. The cursor position will be at the byte after the 0xFF byte.
fn read_until_ff<T>(tty: &mut T) -> Result<(), io::Error>
where
    T: Read,
{
    let mut buf = [0; BUF_SIZE];
    loop {
        tty.read_exact(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        print!("{}", text);
        if buf.contains(&0xFF) {}
    }

    Ok(())
}
