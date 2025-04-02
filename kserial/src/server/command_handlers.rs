use crate::common::{commands::StringPacket, PacketContents};

use super::{read_packet, serial_stream::SerialStream};

pub type Command = fn(u8, &mut SerialStream) -> Result<(), std::io::Error>;

static COMMANDS: [Command; 255] = {
    let mut commands = [invalid as Command; 255];

    commands[StringPacket::ID as usize] = print_str as Command;

    commands
};

fn invalid(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let ich = i as char;
    print!("Invalid command: {ich} (0x{:02X}).", i);
    Ok(())
}

fn print_str(cmd: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let data = read_packet::<StringPacket>(cmd, stream)?;
    print!(
        "{}",
        data.payload()
            .data
            .try_to_string()
            .ok_or(std::io::ErrorKind::InvalidData)?
    );
    Ok(())
}

pub fn handle_command(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    COMMANDS[i as usize](i, stream)
}
