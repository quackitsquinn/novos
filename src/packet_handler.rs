use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
};

pub fn run(pty: &PathBuf) {
    let mut file = OpenOptions::new().read(true).write(true).open(pty).unwrap();
    // Send our version of SYN to the other end
    assert!(file.write(&[0xFF]).unwrap() == 1);

    print_read_until_ff(&mut file);
}

/// Read from the file until we reach 0xFF. Returns the remaining bytes after 0xFF.
fn print_read_until_ff(file: &mut std::fs::File) -> Vec<u8> {
    let mut buf = [0; 8];

    loop {
        file.read_exact(&mut buf).unwrap();

        for i in 0..8 {
            if buf[i] == 0xFF {
                // Print the bytes before 0xFF
                for j in 0..i {
                    print!("{}", buf[j] as char);
                }
                // Return the bytes after 0xFF
                let mut remaining = Vec::new();
                for j in i + 1..8 - i {
                    remaining.push(buf[j]);
                }
                return remaining;
            }
        }

        print!("{}", buf[0] as char);
    }
}
