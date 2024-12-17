use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
};

use kserial::server::Server;

pub fn run(pty: &PathBuf) {
    Server::new(&pty).unwrap().run().unwrap();
}
