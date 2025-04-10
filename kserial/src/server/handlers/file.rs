use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    sync::Mutex,
};

use lazy_static::lazy_static;

use crate::{
    common::commands::{FileFlags, OpenFile},
    server::{read_packet, serial_stream::SerialStream},
};

struct FileData {
    file_table: HashMap<u64, File>,
}

lazy_static! {
    static ref FILE_DATA: Mutex<FileData> = Mutex::new(FileData {
        file_table: HashMap::new(),
    });
}

pub fn open_file(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let data = read_packet::<OpenFile>(i, stream)?;
    let cmd = data.payload();
    let mut opts = OpenOptions::new();
    opts.write(cmd.flags.contains(FileFlags::WRITE));
    opts.read(cmd.flags.contains(FileFlags::READ));
    opts.create(cmd.flags.contains(FileFlags::CREATE));
    opts.append(cmd.flags.contains(FileFlags::APPEND));

    Ok(())
}
