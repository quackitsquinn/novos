use std::{
    env,
    fs::File,
    io::{stdout, BufRead, BufReader, BufWriter, Read, Write},
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
};

use novos::Config;

pub fn main() {
    let cfg = Config::default();
    cfg.run();
}
