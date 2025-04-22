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
            CloseFile, CloseFileResponse, FileFlags, FileResponse, IOError, OpenFile, WriteFile,
            WriteFileResponse,
        },
        PacketContents,
    },
    server::serial_stream::SerialStream,
};

struct FileCommandState {
    open_files: HashSet<i32>,
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
            let raw_fd = file.into_raw_fd();
            let handle = file_data.open_files.insert(raw_fd);
            if !handle {
                panic!("Is this infallible? I don't know. If this ever panics, please report it.");
            }
            res = FileResponse::new(raw_fd);
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

pub fn close_file(i: u8, stream: &mut SerialStream) -> Result<(), std::io::Error> {
    let data = stream.read_packet::<CloseFile>(i)?;
    let cmd = data.payload();
    let mut file_data = FILE_DATA.lock().unwrap();
    let handle = file_data.open_files.take(&cmd.handle.inner());
    if handle.is_none() {
        stream.write_packet(&CloseFileResponse::new(IOError::INVALID_HANDLE).into_packet())?;
        stream.get_inner().flush()?;
        return Ok(());
    }
    let handle = handle.unwrap();
    let file = unsafe { std::fs::File::from_raw_fd(handle) };
    let res = match file.sync_all() {
        Ok(_) => CloseFileResponse::new(IOError::OK),
        Err(e) => CloseFileResponse::new(IOError::from_io_err(e)),
    };
    stream.write_packet(&res.into_packet())?;
    stream.get_inner().flush()?;
    drop(file);
    Ok(())
}
