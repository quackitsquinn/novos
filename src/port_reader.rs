use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

const READ_FILE_COMMAND: u8 = 0x01;

pub fn run(pty: &Path) {
    let pty = pty
        .to_str()
        .expect("Failed to convert pty to string")
        .to_string()
        .trim_matches('"')
        .to_string()
        .parse::<PathBuf>()
        .expect("Infallible");
    println!("Opening pty: {:?}", pty);
    //let mut file = File::open(pty).expect("Failed to open pty");
    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(&pty)
        .expect(format!("Failed to open pty: {:?}", &pty).as_str());
    fs::create_dir("output").unwrap_or_default();
    thread::spawn(move || run_inner(&mut file));
}

fn run_inner(ptr: &mut File) {
    return;
    println!("Initialized port reader");
    let mut buf = [0u8; 8];
    let mut out = File::create("output.txt").expect("Failed to create output file");
    let mut raw_out = File::create("COM2.out").expect("Failed to create raw output file");
    loop {
        if let Ok(n) = ptr.read(&mut buf) {
            raw_out.write(&buf[..n]).expect("Failed to write raw byte");
        }
    }
    loop {
        match ptr.read(&mut buf) {
            Ok(0) => {
                println!("No bytes received");
            }
            Ok(_) => {
                println!("Received byte: {}", buf[0]);
                out.write(&buf).expect("Failed to write byte");
                // Do something with the byte
                match buf[0] {
                    READ_FILE_COMMAND => {
                        read_file(ptr);
                    }
                    _ => {
                        // Handle unknown command
                        println!("Unknown command: {}", buf[0]);
                    }
                }
            }
            Err(e) => {
                // Handle error
                println!("Port Error: {:?}", e);
            }
        }
    }
}

fn read_file(file: &mut File) {
    let mut name_len = [0u8; 1];
    file.read_exact(&mut name_len)
        .expect("Failed to read name length");
    let name_len = name_len[0] as usize;
    let mut name = vec![0u8; name_len];
    file.read_exact(&mut name).expect("Failed to read name");
    let name = String::from_utf8(name).expect("Failed to convert name to string");
    println!("Reading file: {}", name);
    let mut data_len = [0u8; 4];
    let mut buf = [0u8; 1];
    for i in 0..4 {
        while let Err(e) = file.read_exact(&mut buf) {
            eprintln!("Unable to read byte; waiting 1ms ({})", e);
        }
        data_len[i] = buf[0];
    }
    eprintln!("Data len: {:?}", data_len);
    let data_len = u32::from_le_bytes(data_len);
    let mut data = vec![0u8; data_len as usize];
    file.read_exact(&mut data).expect("Failed to read data");
    fs::write(format!("output/{}", name), data).expect("Failed to write file");
}
