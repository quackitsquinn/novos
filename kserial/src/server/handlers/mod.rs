use crate::common::{
    commands::{CloseFile, OpenFile, StringPacket, WriteFile},
    PacketContents,
};

use super::{packet_error::PacketError, serial_stream::SerialStream};

mod file;

pub type PacketResult = Result<(), PacketError>;
pub type Command = fn(u8, &mut SerialStream) -> PacketResult;

static COMMANDS: [Command; 255] = {
    let mut commands = [invalid as Command; 255];

    commands[StringPacket::ID as usize] = print_str as Command;
    commands[OpenFile::ID as usize] = file::open_file as Command;
    commands[WriteFile::ID as usize] = file::write_file as Command;
    commands[CloseFile::ID as usize] = file::close_file as Command;
    commands[0xFE] = echo as Command;

    commands
};

fn invalid(_i: u8, _: &mut SerialStream) -> PacketResult {
    Err(PacketError::InvalidCommand)
}

fn print_str(cmd: u8, stream: &mut SerialStream) -> PacketResult {
    let data = stream.read_packet::<StringPacket>(cmd)?;
    write!(stream.output(), "{}", data.payload().as_str())?;
    Ok(())
}

fn echo(cmd: u8, stream: &mut SerialStream) -> PacketResult {
    let data = stream.read_packet::<StringPacket>(cmd)?;
    stream.write_packet(&data)?;
    stream.get_inner().flush()?;
    Ok(())
}

pub fn handle_command(i: u8, stream: &mut SerialStream) -> Result<(), PacketError> {
    COMMANDS[i as usize](i, stream)
}
