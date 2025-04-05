use core::pin::Pin;
use std::{
    fs::File,
    io::{self, Read, Write},
};

use copy_rw::CopiedReadWrite;
use serial_stream::SerialStream;

use crate::common::{packet::Packet, PacketContents, PACKET_MODE_ENTRY_SIG};

mod command_handlers;
mod copy_rw;
pub(crate) mod serial_stream;

pub struct SerialHandler<T>
where
    T: Read + Write,
{
    datastream: CopiedReadWrite<T, File>,
}

impl<T> SerialHandler<T>
where
    T: Read + Write + 'static,
{
    /// Creates a new server with the given path. The path should be a path to a Unix socket.
    pub fn new(stream: T) -> Result<Self, io::Error> {
        let read_dump = File::create("output/serial_read.raw").map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create read dump: {}", e),
            )
        })?;
        let write_dump: File = File::create("output/serial_write.raw").map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create write dump: {}", e),
            )
        })?;
        Ok(SerialHandler {
            datastream: CopiedReadWrite {
                read_dump: read_dump,
                write_dump: write_dump,
                inner: stream,
            },
        })
    }

    pub fn run(self) -> Result<(), io::Error> {
        println!("Server started");
        let mut stream = SerialStream::new(self.datastream);
        loop {
            println!("Waiting for packet mode entry signature...");
            read_until_signature(&mut stream, &PACKET_MODE_ENTRY_SIG)?;
            println!("Entered packet mode, starting to process packets.");
            run_packet_mode(&mut stream)?;
        }

        Ok(())
    }
}

pub(crate) fn read_packet<C: PacketContents>(
    cmd_id: u8,
    stream: &mut SerialStream,
) -> Result<Packet<C>, io::Error> {
    let checksum = stream.read_ty::<u8>()?;
    let packet = stream.read_ty::<C>()?;
    let full = Packet::from_raw_parts(cmd_id, checksum, packet).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Checksum mismatch, expected 0 but got {}",
            packet.checksum()
        ),
    ))?;
    Ok(full)
}

fn read_until_signature(stream: &mut SerialStream, signature: &[u8]) -> Result<(), io::Error> {
    let mut sig_index = 0;

    loop {
        let mut byte = [0; 1];
        let res = stream.get_inner().read_exact(&mut byte);

        if let Err(e) = res {
            if e.kind() == io::ErrorKind::Interrupted || e.kind() == io::ErrorKind::TimedOut {
                // Try again if the read was interrupted or timed out.
                continue;
            } else {
                // Some other IO error occurred.
                return Err(e);
            }
        }

        let as_char = byte[0] as char;

        print!("{}", as_char);

        if byte[0] == signature[sig_index] {
            // Check if we matched the current byte in the signature.
            if sig_index + 1 == signature.len() {
                // We matched the full signature, return success.
                println!("Matched packet mode entry signature.");
                return Ok(());
            } else {
                // Move to the next byte in the signature.
                sig_index += 1;
            }
        } else {
            // Check if this character matches the first byte of the signature, otherwise reset the index.
            if byte[0] == signature[0] {
                sig_index = 1; // Start matching from the second byte of the signature.
            } else {
                sig_index = 0; // Reset if it doesn't match the first byte.
            }
        }
    }
}

fn run_packet_mode(stream: &mut SerialStream) -> Result<(), io::Error> {
    loop {
        let cmd_id = stream.read_ty::<u8>()?;
        match command_handlers::handle_command(cmd_id, stream) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to handle command {}: {}", cmd_id, e);
                return Err(e);
            }
        }
    }
}
