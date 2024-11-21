//! The auxiliary serial port is a second serial port that can be used for debugging purposes.
//! The primary intention is to send files to the host machine for debugging purposes.

use core::time::Duration;

use log::info;
use spin::Once;

use crate::{interrupts::hardware::timer::Timer, sprintln, util::OnceMutex};

use super::raw::SerialPort;

/// The port number for the auxiliary serial port
const AUX_SERIAL_PORT: u16 = 0x2f8;
static AUX_PORT: OnceMutex<SerialPort> = OnceMutex::new();

pub fn init_aux_serial() {
    AUX_PORT.init({
        let mut port = unsafe { SerialPort::new(AUX_SERIAL_PORT) };
        port.init();
        port
    });
}

#[derive(thiserror::Error, Debug)]
pub enum AuxError {
    #[error("File name too long")]
    FileNameTooLong,
}

macro_rules! write_bytes {
    ($port:expr, $bytes:expr) => {
        for byte in $bytes {
            $port.send(*byte);
        }
    };
}

const WRITE_FILE_COMMAND: u8 = 0x01;

pub fn send_data(filename: &str, data: &[u8]) -> Result<(), AuxError> {
    let mut port = AUX_PORT.get();
    let data_len = filename.len().to_le_bytes();
    let name_len: u8 = filename
        .len()
        .try_into()
        .map_err(|_| AuxError::FileNameTooLong)?;
    sprintln!("Sending file: {}", filename);
    port.send(WRITE_FILE_COMMAND);
    port.send(name_len);
    write_bytes!(port, filename.as_bytes());
    write_bytes!(port, &data_len);
    write_bytes!(port, data);
    sprintln!("Sent file: {}", filename);

    Ok(())
}
