use std::path::PathBuf;

use kserial::server::Server;

pub fn run(pty: &PathBuf) {
    // Just print the raw output to the console

    Server::new(&pty).unwrap().run().unwrap();
}
