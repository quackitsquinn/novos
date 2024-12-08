//! The auxiliary serial port is a second serial port that can be used for debugging purposes.
//! The primary intention is to send files to the host machine for debugging purposes.

use core::{fmt::Write, time::Duration};

use log::info;
use spin::Once;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{interrupts::hardware::timer::Timer, println, sprint, sprintln, util::OnceMutex};

use super::raw::SerialPort;

/// The port number for the auxiliary serial port
const AUX_SERIAL_PORT: u16 = 0x2f8;
static AUX_PORT: OnceMutex<SerialPort> = OnceMutex::new();

pub fn init_aux_serial() {
    println!("Initializing auxiliary serial port");
    AUX_PORT.init({
        let mut port = unsafe { SerialPort::new(AUX_SERIAL_PORT) };
        port.init();
        port
    });
    println!("Initialized auxiliary serial port");
}

#[derive(thiserror::Error, Debug)]
pub enum AuxError {
    #[error("File name too long")]
    FileNameTooLong,
}

macro_rules! write_bytes {
    ($port:expr, $bytes:expr) => {
        sprint!("Sending bytes: ");
        without_interrupts(|| {
        // Im curious.. what if this is ran with interrupts disabled? Would that fix it being inconsistent?
        for byte in $bytes {
            sprint!("{:x}", *byte);
            $port.send_raw(*byte);
        }
            sprintln!();
        });
    };
}

const WRITE_FILE_COMMAND: u8 = 0x01;

pub fn send_data(filename: &str, data: &[u8]) -> Result<(), AuxError> {
    let mut port = AUX_PORT.get();
    let data_len = (data.len() as u64).to_le_bytes();
    let name_len: u8 = filename
        .len()
        .try_into()
        .map_err(|_| AuxError::FileNameTooLong)?;
    sprintln!(
        "Sending file: {} with length {} ({:?})",
        filename,
        data.len(),
        data_len
    );

    port.send_raw(WRITE_FILE_COMMAND);
    port.send_raw(name_len);
    write_bytes!(port, filename.as_bytes());
    sprintln!("Sent filename: {}.. Sending data length", filename);
    write_bytes!(port, &data_len);
    sprintln!("Sent data length: {}", data.len());
    write_bytes!(port, data);
    sprintln!("Sent file: {}", filename);

    Ok(())
}
