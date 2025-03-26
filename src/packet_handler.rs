use std::path::PathBuf;

use kserial::server::SerialHandler;

pub fn run(pty: &PathBuf) {
    // Just print the raw output to the console

    SerialHandler::new(&pty).unwrap().run().unwrap();
}
