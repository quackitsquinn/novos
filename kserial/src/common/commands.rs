use core::fmt::Arguments;

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
