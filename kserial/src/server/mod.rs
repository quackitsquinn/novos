use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    thread,
};

pub struct Server {
    listener: UnixListener,
    tty: UnixStream,
}

impl Server {
    /// Creates a new server with the given path. The path should be a path to a Unix socket.
    pub fn new(path: &Path) -> Result<Self, io::Error> {
        fs::remove_file(path);
        fs::create_dir("output");
        for i in 0..10 {
            let listener = match UnixListener::bind(path) {
                Ok(tty) => tty,
                Err(e) => {
                    if i == 9 {
                        return Err(e);
                    }
                    println!("Failed to bind to socket, retrying in 500ms");
                    thread::sleep(std::time::Duration::from_millis(500));
                    continue;
                }
            };

            let (tty, _) = listener.accept()?;

            return Ok(Server {
                listener: listener,
                tty,
            });
        }
        unreachable!()
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        // First off, we read all bytes from the tty until we encounter a 0xFF byte. This is the handshake byte, so once we read it, we switch to packet mode.
        read_until_ten_ff(&mut self.tty)?;
        let mut buf = [0; 1];
        loop {
            if let Err(e) = self.tty.read_exact(&mut buf) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(e);
            }
            match buf[0] {
                // Both commands are just null-terminated strings, so we can handle them the same way.
                0x00 | 0x01 => handle_write_string_command(&mut self.tty)?,
                0x02 => handle_send_file_command(&mut self.tty)?,
                0xFF => break,
                _ => panic!("Invalid command byte"),
            }
        }
        Ok(())
    }
}

fn handle_write_string_command(read: &mut dyn Read) -> io::Result<()> {
    let mut buf = [0; 1];
    let mut string = read_nul_terminated_str(read)?;
    print!("{}", string);
    flush();
    Ok(())
}

fn handle_send_file_command(read: &mut dyn Read) -> io::Result<()> {
    println!("Handling send file command");
    let mut buf = [0; 1];
    let mut filename = read_nul_terminated_str(read)?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("output/{}", filename))?;

    let mut len_buf = [0; 4];
    read.read_exact(&mut len_buf)?;

    let len = u32::from_le_bytes(len_buf) as usize;
    println!("File length: {}", len);

    let mut buf = vec![0; len];
    read.read_exact(&mut buf)?;

    file.write_all(&buf)?;

    Ok(())
}

#[inline]
fn flush() {
    io::stdout().flush().unwrap();
}

fn read_until_ten_ff(read: &mut dyn Read) -> io::Result<()> {
    let mut buf = [0; 1];
    let mut count = 0;
    loop {
        read.read_exact(&mut buf)?;
        if buf[0] == 0xFF {
            count += 1;
            if count == 10 {
                break;
            }
        } else {
            count = 0;
        }
    }
    Ok(())
}

fn read_nul_terminated_str(read: &mut dyn Read) -> io::Result<String> {
    let mut buf = [0; 1];
    let mut string = String::new();
    loop {
        read.read_exact(&mut buf)?;
        if buf[0] == 0 {
            break;
        }
        string.push(buf[0] as char);
    }
    Ok(string)
}
