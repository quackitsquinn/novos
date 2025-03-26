use std::io::{Read, Write};

use super::Stream;

pub type Command = fn(u8, &mut Stream);

static COMMANDS: [Command; 255] = {
    let mut commands = [invalid as Command; 255];

    commands
};

fn invalid(i: u8, stream: &mut Stream) {
    let ich = i as char;
    print!("{}", ich);
}

pub fn handle_command(i: u8, stream: &mut Stream) {
    if i < 255 {
        COMMANDS[i as usize](i, stream);
    } else {
        invalid(i, stream);
    }
}
