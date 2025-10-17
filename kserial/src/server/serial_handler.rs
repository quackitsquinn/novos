//! Handler for serial connections.
use core::fmt::Debug;
use std::{
    fs::File,
    io::{self, Read, Write},
};

use crate::common::PACKET_MODE_ENTRY_SIG;

use super::{copy_rw::CopiedReadWrite, handlers, serial_stream::SerialStream};

/// A handler for serial connections that processes packets.
pub struct SerialHandler<T>
where
    T: Read + Write,
{
    datastream: CopiedReadWrite<T, File>,
    string_output: Box<dyn Write>,
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
            string_output: Box::new(io::stdout()),
        })
    }

    /// Sets the output for string packets to the given writer.
    pub fn with_output<W>(self, output: W) -> Self
    where
        W: Write + 'static,
    {
        SerialHandler {
            datastream: self.datastream,
            string_output: Box::new(output),
        }
    }

    /// Runs the serial handler, processing incoming packets.
    pub fn run(self) -> Result<(), io::Error> {
        println!("Server started");
        let mut stream = SerialStream::new(self.datastream, self.string_output);

        println!("Waiting for packet mode entry signature...");
        loop {
            read_until_signature(&mut stream, &PACKET_MODE_ENTRY_SIG)?;
            println!("Entered packet mode, starting to process packets.");
            let text = run_packet_mode(&mut stream)?;
            let text = String::from_utf8_lossy(&text);
            print!("{}", text);
        }
    }
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
            // Print the bytes we matched so far.
            if sig_index > 0 {
                for i in 0..sig_index {
                    print!("{}", signature[i] as char);
                }
            }
            // Check if this character matches the first byte of the signature, otherwise reset the index.
            if byte[0] == signature[0] {
                sig_index = 1; // Start matching from the second byte of the signature.
            } else {
                sig_index = 0; // Reset if it doesn't match the first byte.
            }
            print!("{}", as_char);
        }
    }
}

/// Maximum number of continuous invalid bytes before exiting packet mode.
pub const MAX_CONTINUOUS_INVALID_BYTES: usize = 0x10;

fn run_packet_mode(stream: &mut SerialStream) -> Result<Vec<u8>, io::Error> {
    let mut invalid_bytes = Vec::new();
    loop {
        // SAFETY: All we need here is the byte, no header or anything else.
        let cmd_id = unsafe { stream.read_ty::<u8>()? };
        match handlers::handle_command(cmd_id, stream) {
            Ok(_) => {
                invalid_bytes.clear();
            }
            Err(e) => {
                if e.is_invalid_command() {
                    invalid_bytes.push(cmd_id);
                }
                if e.is_invalid_checksum() {
                    println!("Invalid checksum for command {cmd_id}");
                }
                if e.is_io_error() {
                    println!("IO error for command {cmd_id}: {}", e.io_error().unwrap());
                }
            }
        }

        if invalid_bytes.len() > MAX_CONTINUOUS_INVALID_BYTES {
            println!("Too many invalid bytes, exiting packet mode.");
            return Ok(invalid_bytes);
        }
    }
}

impl<T: Read + Write + Debug> Debug for SerialHandler<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialHandler")
            .field("datastream", &self.datastream.inner)
            .finish()
    }
}
