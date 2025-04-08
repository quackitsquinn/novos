use crate::common::{commands::StringPacket, PacketContents};

use super::{read_packet, serial_stream::SerialStream};

pub type Command = fn(u8, &mut SerialStream) -> Result<(), std::io::Error>;

static COMMANDS: [Command; 255] = {
    let mut commands = [invalid as Command; 255];

    commands[StringPacket::ID as usize] = print_str as Command;
    commands[0xFE] = echo as Command;

    commands
};

fn invalid(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let ich = i as char;
    print!("Invalid command: {ich} (0x{:02X}).", i);
    Ok(())
}

fn print_str(cmd: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let data = read_packet::<StringPacket>(cmd, stream)?;
    print!("{}", data.payload().as_str());
    Ok(())
}

fn echo(cmd: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let data = read_packet::<StringPacket>(cmd, stream)?;
    stream.write_ty(&data)?;
    stream.get_inner().flush()?;
    Ok(())
}

pub fn handle_command(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    COMMANDS[i as usize](i, stream)
}
