use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
};

mod command_handlers;

pub trait RWStream: Read + Write {}

impl<T: Read + Write> RWStream for T {}

pub type Stream = Box<dyn RWStream>;

pub struct SerialHandler<T>
where
    T: RWStream,
{
    datastream: OpaqueCopyRead<T, File>,
}

impl<T> SerialHandler<T>
where
    T: Read + Write,
{
    /// Creates a new server with the given path. The path should be a path to a Unix socket.
    pub fn new(stream: T) -> Result<Self, io::Error> {
        let dump = File::create("output/serial.log")?;
        Ok(SerialHandler {
            datastream: OpaqueCopyRead {
                dump,
                inner: stream,
            },
        })
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        println!("Server started");

        Ok(())
    }
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
