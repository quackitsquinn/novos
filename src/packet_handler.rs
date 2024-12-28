use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    thread,
};

use kserial::server::Server;

pub fn run(pty: &PathBuf) {
    // Just print the raw output to the console

    Server::new(&pty).unwrap().run().unwrap();
}
