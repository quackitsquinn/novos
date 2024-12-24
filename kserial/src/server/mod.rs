use std::{
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    thread,
};

const BUF_SIZE: usize = 16;

pub struct Server {
    tty: UnixStream,
}

impl Server {
    /// Creates a new server with the given path. The path should be a path to a Unix socket.
    pub fn new(path: &Path) -> Result<Self, io::Error> {
        for i in 0..10 {
            let tty = match UnixStream::connect(path) {
                Ok(tty) => tty,
                Err(e) => {
                    if i == 9 {
                        return Err(e);
                    }
                    thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };
            return Ok(Self { tty });
        }
        unreachable!()
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        // First off, we read all bytes from the tty until we encounter a 0xFF byte. This is the handshake byte, so once we read it, we switch to packet mode.
        read_until_ff(&mut self.tty)?;
        println!("Got 0xFF byte, switching to packet mode");
        // dude idfk spam 0xFF bytes. The qemu serial port is weird, and like, incredibly unreliable.
        // I have tried so many things to do this properly, including using a second port to read commands but LITERALLY NOTHING WORKS
        thread::sleep(std::time::Duration::from_secs(1));
        self.tty.write_all(&[0xFF])?;

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
        if buf.contains(&0xFF) {
            break;
        }
    }

    Ok(())
}
