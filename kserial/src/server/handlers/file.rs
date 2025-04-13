use core::mem::transmute;
use std::{collections::HashSet, fs::OpenOptions, os::fd::IntoRawFd, path::PathBuf, sync::Mutex};

use lazy_static::lazy_static;

use crate::{
    common::{
        commands::{FileFlags, FileResponse, OpenFile},
        PacketContents,
    },
    server::{read_packet, serial_stream::SerialStream},
};

struct FileCommandState {
    open_files: HashSet<u64>,
}

lazy_static! {
    static ref FILE_DATA: Mutex<FileCommandState> = Mutex::new(FileCommandState {
        open_files: HashSet::new(),
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
    let file = opts.open(PathBuf::from("output/").join(&*cmd.path));

    let res;

    match file {
        Ok(file) => {
            let mut file_data = FILE_DATA.lock().unwrap();
            let raw_fd_u64 = unsafe { transmute::<_, u32>(file.into_raw_fd()) } as u64;
            let handle = file_data.open_files.insert(raw_fd_u64);
            if !handle {
                panic!("Is this infallible? I don't know. If this ever panics, please report it.");
            }
            res = FileResponse::new(raw_fd_u64);
        }
        Err(e) => {
            res = FileResponse::from_io_err(e);
        }
    }

    stream.write_ty(&res.into_packet())?;

    Ok(())
}
