use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
};

pub struct Server {
    #[allow(dead_code)]
    listener: UnixListener,
    unix_term_stream: OpaqueCopyRead<UnixStream, File>,
}

impl Server {
    /// Creates a new server with the given path. The path should be a path to a Unix socket.
    pub fn new(path: &Path) -> Result<Self, io::Error> {
        let _ = fs::remove_file(path);
        let _ = fs::create_dir("output");
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

            let (stream, addr) = listener.accept()?;
            println!("Connected to {:?}", addr);

            return Ok(Server {
                listener: listener,
                unix_term_stream: OpaqueCopyRead {
                    dump: File::create("output/tty")?,
                    inner: stream,
                },
            });
        }
        unreachable!();
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        println!("Server started");
        // First off, we read all bytes from the tty until we encounter a 0xFF byte. This is the handshake byte, so once we read it, we switch to packet mode.
        read_until_ten_ff(&mut self.unix_term_stream)?;
        let mut buf = [0; 1];

        loop {
            if let Err(e) = self.unix_term_stream.read_exact(&mut buf) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(e);
            }
            match buf[0] {
                // Both commands are just null-terminated strings, so we can handle them the same way.
                0x00 | 0x01 => handle_write_string_command(&mut self.unix_term_stream)?,
                0x02 => handle_send_file_command(&mut self.unix_term_stream)?,
                0x04 => handle_create_incremental_channel_command(&mut self.unix_term_stream)?,
                0x05 => handle_send_incremental_data_command(&mut self.unix_term_stream)?,
                0xFF => break,
                _ => eprintln!("Invalid byte: {}", buf[0]),
            }
        }
        Ok(())
    }
}

fn handle_write_string_command(read: &mut dyn Read) -> io::Result<()> {
    let string = read_nul_terminated_str(read)?;
    print!("{}", string);
    flush();
    Ok(())
}

fn handle_send_file_command(read: &mut dyn Read) -> io::Result<()> {
    println!("Handling send file command");
    let filename = read_nul_terminated_str(read)?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("output/{}", filename))?;

    let buf = read_u32_len_data(read)?;

    file.write_all(&buf)?;

    Ok(())
}

fn read_u32_len_data(read: &mut dyn Read) -> io::Result<Vec<u8>> {
    let mut len_buf = [0; 4];
    read.read_exact(&mut len_buf)?;

    let len = u32::from_le_bytes(len_buf) as usize;
    println!("File length: {}", len);

    let mut all = vec![0; len];
    let mut buf = [0; 2 << 20];
    let mut total = 0;
    let full_block_count = len / buf.len();
    let remainder = len % buf.len();

    for _ in 0..full_block_count {
        read.read_exact(&mut buf)?;
        all[total..total + buf.len()].copy_from_slice(&buf);
        total += 1;
        println!("Read block {} out of {}", total, full_block_count);
    }

    if remainder > 0 {
        read.read_exact(&mut buf[..remainder])?;
        all[total..total + remainder].copy_from_slice(&buf[..remainder]);
    }

    println!("Read {} bytes", total + remainder);
    Ok(all)
}

static CHANNELS: OnceLock<Mutex<HashMap<String, (String, usize)>>> = OnceLock::new();

fn channels<'a>() -> (MutexGuard<'a, HashMap<String, (String, usize)>>, bool) {
    let mut did_init = false;
    let channels = CHANNELS.get_or_init(|| {
        did_init = true;
        Mutex::new(HashMap::new())
    });
    (channels.lock().unwrap(), did_init)
}

fn handle_create_incremental_channel_command(read: &mut dyn Read) -> io::Result<()> {
    let channel = read_nul_terminated_str(read)?;
    let format = read_nul_terminated_str(read)?;
    println!("Creating incremental channel: {}", channel);
    let (mut channels, _) = channels();
    if channels.contains_key(&channel) {
        eprintln!("Channel already exists");
        return Ok(());
    }
    channels.insert(channel, (format, 0));
    Ok(())
}

fn handle_send_incremental_data_command(read: &mut dyn Read) -> io::Result<()> {
    let channel = read_nul_terminated_str(read)?;
    let data = read_u32_len_data(read)?;

    let (mut channels, did_init) = channels();
    if did_init {
        eprintln!("Channel does not exist");
        return Ok(());
    }
    let (format, id) = channels.get_mut(&channel).ok_or_else(|| {
        eprintln!("Channel does not exist");
        io::Error::new(io::ErrorKind::NotFound, "Channel does not exist")
    })?;
    let filename = format.replace("{{ID}}", &id.to_string());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("output/{}", filename))?;
    file.write_all(&data)?;
    *id += 1;
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

/// A wrapper around a Read that copies all data read from it to another Write.
struct OpaqueCopyRead<T, D>
where
    T: Read,
    D: Write,
{
    dump: D,
    inner: T,
}

impl<T, D> Read for OpaqueCopyRead<T, D>
where
    T: Read,
    D: Write,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let res = self.inner.read(buf)?;
        if res == 0 {
            return Ok(0);
        }
        self.dump.write_all(&buf[..res])?;
        Ok(res)
    }
}
