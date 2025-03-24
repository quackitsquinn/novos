use core::fmt::Arguments;

/// Commands that can be sent to the serial port. This is used for structured communication.
/// Each command has a corresponding handler defined in this module.
/// Modules do *not* send the command id, only the data.
#[repr(u8)]
#[must_use = "Commands must be sent to the serial port to have any effect."]
pub enum Command<'a> {
    /// Write a string to the serial port.
    WriteString(&'a str) = 0,
    /// Write a set of arguments to the serial port. Different from WriteString, because the implementation uses a null-terminated string.
    WriteArguments(&'a Arguments<'a>),
    /// Send a file over the serial port. (filename, contents)
    SendFile(&'a str, &'a [u8]),
    /// Disable packet support. YOU CAN NOT RE-ENABLE PACKET SUPPORT AT THIS POINT.
    DisablePacketSupport,
    /// Initialize incremental send mode. (channel name, file_format)
    /// File format will replace **id** with the number of the file.
    InitIncrementalSend(&'a str, &'a str),
    /// Send incremental data. (channel name, data)
    SendIncrementalData(&'a str, &'a [u8]),
}

impl Command<'_> {
    /// Get the command id.
    pub fn id(&self) -> u8 {
        // TODO: If more commands are added, refactor this all into a proc macro.
        match self {
            Command::WriteString(_) => 0,
            Command::WriteArguments(_) => 1,
            Command::SendFile(_, _) => 2,
            Command::DisablePacketSupport => 3,
            Command::InitIncrementalSend(_, _) => 4,
            Command::SendIncrementalData(_, _) => 5,
        }
    }
}
