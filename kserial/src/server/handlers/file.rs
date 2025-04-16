use core::mem::transmute;
use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::Write,
    os::fd::{FromRawFd, IntoRawFd},
    path::PathBuf,
    sync::Mutex,
};

use lazy_static::lazy_static;

use crate::{
    common::{
        commands::{
            FileFlags, FileResponse, IOError, OpenFile, OsError, WriteFile, WriteFileResponse,
        },
        PacketContents,
    },
    server::serial_stream::SerialStream,
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
    let data = stream.read_packet::<OpenFile>(i)?;
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

    stream.write_packet(&res.into_packet())?;
    stream.get_inner().flush()?;
    Ok(())
}

pub fn write_file(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let data = stream.read_packet::<WriteFile>(i)?;
    let cmd = data.payload();
    let file_data = FILE_DATA.lock().unwrap();
    if !file_data.open_files.contains(&cmd.file().inner()) {
        stream.write_packet(&WriteFileResponse::err(IOError::INVALID_HANDLE).into_packet())?;
        stream.get_inner().flush()?;
        return Ok(());
    }

    let mut file = unsafe { std::fs::File::from_raw_fd(cmd.file().inner() as i32) };

    let res = match file.write_all(cmd.data()) {
        Ok(_) => WriteFileResponse::ok(),
        Err(e) => WriteFileResponse::err(IOError::from_io_err(e)),
    };

    stream.write_packet(&res.into_packet())?;
    stream.get_inner().flush()?;

    // Not our responsibility to close the file
    let _ = file.into_raw_fd();

    Ok(())
}
