use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    os,
};

use proc_macro::TokenStream;

pub fn download_and_parse_pci_ids() -> Vec<Vendor> {
    let mut file = get_pci_ids();
    let vendors = parse_pci_ids(file);
    return vendors;
}

fn get_pci_ids() -> File {
    fn download_cache() -> File {
        // Download the file from the URL
        let url = "https://pci-ids.ucw.cz/v2.2/pci.ids";

        // Ok so, cargo is not liking reqwest or ureq, so we are going to have to do this manually.
        // We are going to use curl and fallback to wget if curl is not available.

        // Check if curl is available
        let output = std::process::Command::new("curl").arg("--version").output();
        if output.is_ok() {
            // Use curl to download the file
            let file = File::create("target/pci.ids").unwrap();
            std::process::Command::new("curl")
                .arg(url)
                .stdout(file)
                .output()
                .unwrap();
        }
        // Use wget to download the file
        let file = File::create("target/pci.ids").unwrap();
        std::process::Command::new("wget")
            .arg(url)
            .stdout(file)
            .output()
            .unwrap();
        return File::open("target/pci.ids").unwrap();
    }

    if let Ok(file) = File::open("target/pci.ids") {
        return file;
    }
    // Check if we are on a unix-like system
    if cfg!(unix) {
        // Check if the file exists (Some distributions do not cache the file)
        if let Ok(file) = File::open("/usr/share/hwdata/pci.ids") {
            return file;
        } else {
            return download_cache();
        }
    }
    return download_cache();
}

#[derive(Debug)]
pub struct Vendor {
    pub id: u16,
    pub name: String,
    pub devices: Vec<Device>,
}

impl Vendor {
    pub fn new(id: u16, name: String, devices: Vec<Device>) -> Self {
        Self {
            id,
            name: name.trim().to_string(),
            devices,
        }
    }
}
#[derive(Debug)]
pub struct Device {
    pub id: u16,
    pub name: String,
    pub subdevices: Vec<SubDevice>,
}

impl Device {
    pub fn new(id: u16, name: String, subdevices: Vec<SubDevice>) -> Self {
        Self {
            id,
            name: name.trim().to_string(),
            subdevices,
        }
    }
}
#[derive(Debug)]
pub struct SubDevice {
    pub subvendor: u16,
    pub id: u16,
    pub name: String,
}
impl SubDevice {
    pub fn new(subvendor: u16, id: u16, name: String) -> Self {
        Self {
            subvendor,
            id,
            name: name.trim().to_string(),
        }
    }
}

fn parse_pci_ids(file: File) -> Vec<Vendor> {
    let mut vendors = vec![];
    let mut current_vendor: Option<Vendor> = None;
    let mut current_device: Option<Device> = None;
    let mut lines = BufReader::new(file)
        .lines()
        .map(|l| l.unwrap())
        .filter(|l| !l.starts_with('#') && !l.is_empty());

    fn read_hex(line: &str) -> u16 {
        u16::from_str_radix(&line[0..4], 16).unwrap()
    }

    for line in lines {
        if line.starts_with("C ") {
            // We have made it to class codes. Break.
            break;
        }

        if !line.starts_with("\t") {
            if let Some(device) = current_device.take() {
                current_vendor
                    .as_mut()
                    .expect("Vendor should be set")
                    .devices
                    .push(device);
            }
            if let Some(vendor) = current_vendor {
                vendors.push(vendor);
            }
            current_vendor = Some(Vendor::new(read_hex(&line), line[5..].to_string(), vec![]));
            continue;
        }

        if line.starts_with("\t\t") {
            let mut device = current_device.as_mut().expect("Device should be set");
            let subvendor = read_hex(&line[2..6]);
            let id = read_hex(&line[7..11]);
            device
                .subdevices
                .push(SubDevice::new(subvendor, id, line[12..].to_string()));
            continue;
        }

        if let Some(device) = current_device.take() {
            current_vendor
                .as_mut()
                .expect("Vendor should be set")
                .devices
                .push(device);
        }

        let id = read_hex(&line[1..5]);
        current_device = Some(Device::new(id, line[6..].to_string(), vec![]));
    }

    if let Some(device) = current_device {
        current_vendor
            .as_mut()
            .expect("Vendor should be set")
            .devices
            .push(device);
    }
    if let Some(vendor) = current_vendor {
        vendors.push(vendor);
    }

    vendors
}
