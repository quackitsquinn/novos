use std::{
    env,
    io::{self, BufRead},
    path::PathBuf,
    process::Stdio,
};

use jzon::JsonValue;
use kbuild::config::Config as KConfig;
use novos::Config;

// TODO: Figure out how to pass test args to the test build command to be able to run specific tests
fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let test_args = args[1..].to_vec();

    let kernel_path = build_tests(test_args);

    println!("Built tests at {:?}", kernel_path);
    if kernel_path.1 || env::var("REBUILD").is_ok() {
        println!("Creating iso");
        make_test_iso(kernel_path.0);
    }
    println!("Running tests");

    let mut cfg = Config::default();
    cfg.iso = "boot_images/kernel_tests.iso".to_string();
    cfg.dev_exit = true;
    cfg.graphics = false;
    cfg.serial.clear();
    cfg.serial.push("chardev:output".to_string());
    //cfg.wait_for_debugger = true;
    cfg.run();
}

/// Builds the test kernel and returns the path to the kernel binary
/// Returns the path to the kernel binary, and if it was freshly built
fn build_tests(test_args: Vec<String>) -> (PathBuf, bool) {
    let cmd = std::process::Command::new("cargo")
        .args(&[
            "build",
            "--tests",
            "--message-format=json-diagnostic-rendered-ansi",
        ])
        .args(&test_args)
        .stdout(Stdio::piped())
        .current_dir("kernel")
        .spawn()
        .unwrap();
    let mut last_artifact = JsonValue::Null;
    let reader = std::io::BufReader::new(cmd.stdout.unwrap());
    for res in reader.split(b'\n') {
        if let Err(e) = res {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                break;
            }
            panic!("Error reading cargo output: {}", e);
        }
        let line = res.as_ref().unwrap();
        let line = std::str::from_utf8(&line).unwrap();
        let json = jzon::parse(line).unwrap();
        if json["reason"] == "compiler-artifact" {
            last_artifact = json;
            continue;
        }

        if json["reason"] == "compiler-message" {
            if let JsonValue::String(strn) = &json["message"]["rendered"] {
                print!("{}", strn);
            }
        }

        if json["reason"] == "build-finished" {
            if json["success"] == true {
                let path = last_artifact["executable"].as_str().unwrap();
                let fresh = last_artifact["fresh"].as_bool().unwrap();
                // TODO: For some reason, fresh is always false. This is a temporary fix.
                return (PathBuf::from(path), true);
            }
        }
    }

    panic!("Failed to build tests");
}

fn make_test_iso(kernel_path: PathBuf) {
    let cfg = KConfig::new(
        "target/artifacts".parse().unwrap(),
        kernel_path,
        "boot_cfg/test.conf".parse().unwrap(),
        "boot_images".parse().unwrap(),
        "kernel_tests.iso".parse().unwrap(),
    );

    kbuild::build(&cfg);
}
