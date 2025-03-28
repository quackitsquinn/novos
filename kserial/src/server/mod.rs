use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
};

use crate::common::{packet::Packet, PacketContents};
use crate::server::read_from::ReadFrom;

mod command_handlers;
mod read_from;

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

pub(crate) fn read_packet<C: PacketContents>(
    cmd_id: u8,
    stream: &mut Stream,
) -> Result<Packet<C>, io::Error> {
    let checksum = stream.read_ty::<u8>()?;
    let packet = stream.read_ty::<C>()?;
    let full = Packet::from_raw_parts(cmd_id, checksum, packet).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Checksum mismatch",
    ))?;
    Ok(full)
}
