use core::{
    error::Error,
    fmt::{Arguments, Write},
};

use super::serial::Serial;

/// Commands that can be sent to the serial port. This is used for structured communication.
/// Each command has a corresponding handler defined in this module.
/// Modules do *not* send the command id, only the data.
#[repr(u8)]
pub enum Command<'a> {
    /// Write a string to the serial port.
    WriteString(&'a str) = 0,
    /// Write a set of arguments to the serial port. Different from WriteString, because the implementation uses a null-terminated string.
    WriteArguments(&'a Arguments<'a>),
    /// Send a file over the serial port. (filename, contents)
    SendFile(&'a str, &'a [u8]),
}

impl Command<'_> {
    /// Handle the command.
    pub fn handle(&self, serial: &mut Serial) -> CommandResult {
        let already_in_command = serial.in_command;
        if !serial.in_command {
            unsafe { serial.send_raw(self.id()) };
            serial.in_command = true;
        }
        // TODO: If I need to implement a lot of commands, create a macro to generate like most of this file.
        let res = match self {
            Command::WriteString(s) => write_string(serial, s),
            Command::WriteArguments(args) => write_arguments(serial, args),
            Command::SendFile(filename, contents) => send_file(serial, filename, contents),
        };
        serial.in_command = already_in_command;
        res
    }

    /// Get the command id.
    pub fn id(&self) -> u8 {
        // Because the fields all contain different types, we can't cast the enum to an integer. Instead, we use a match statement to get the id.
        match self {
            Command::WriteString(_) => Command::WriteString as u8,
            Command::WriteArguments(_) => Command::WriteArguments as u8,
            Command::SendFile(_, _) => Command::SendFile as u8,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Null terminator found in string")]
    NullError,
}

type CommandResult = core::result::Result<(), CommandError>;

/// Write a string to the serial port. This is a handler for the WriteString command.
///
fn write_string(serial: &mut Serial, s: &str) -> CommandResult {
    if s.contains('\0') {
        return Err(CommandError::NullError);
    }
    unsafe {
        serial.send_slice_raw(s.as_bytes());
        serial.send_raw(0);
    };
    Ok(())
}

/// Write a set of arguments to the serial port. This is a handler for the WriteArguments command.
fn write_arguments(serial: &mut Serial, args: &Arguments) -> CommandResult {
    // Write the arguments. The nul-safety is handled by the write! macro.
    write!(serial, "{}", args).unwrap();
    // Write a null terminator.
    unsafe { serial.send_raw(0) };
    Ok(())
}

/// Send a file over the serial port. This is a handler for the SendFile command.
fn send_file(serial: &mut Serial, filename: &str, contents: &[u8]) -> CommandResult {
    serial.run_command(Command::WriteString(filename));
    Ok(())
}
