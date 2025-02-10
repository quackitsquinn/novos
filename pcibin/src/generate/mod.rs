use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
};

mod binary_alloc;
mod str_alloc;

pub fn generate(output: &Path) -> Result<(), std::io::Error> {
    let mut file = get_pci_ids();
    let vendors = parse_pci_ids(file);
    Ok(())
}

fn get_pci_ids() -> File {
    fn download_cache() -> File {
        // Download the file from the URL
        let url = "https://pci-ids.ucw.cz/v2.2/pci.ids";

        let mut req = ureq::get(url).call();
        if let Ok(result) = req.as_mut() {
            if !result.status().is_success() {
                panic!("Failed to download pci.ids");
            }
            let mut file = File::create("target/pci.ids").unwrap();
            file.write_all(
                result
                    .body_mut()
                    .read_to_vec()
                    .expect("Failed to read response body")
                    .as_slice(),
            )
            .unwrap();
            return File::open("target/pci.ids").unwrap();
        } else {
            panic!("Failed to download pci.ids");
        }
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
    id: u16,
    vendor_name: String,
    devices: Vec<Device>,
}

impl Vendor {
    pub fn new(id: u16, vendor_name: String, devices: Vec<Device>) -> Self {
        Self {
            id,
            vendor_name,
            devices,
        }
    }
}

#[derive(Debug)]
pub struct Device {
    id: u16,
    device_name: String,
    subdevices: Vec<SubDevice>,
}

impl Device {
    pub fn new(id: u16, device_name: String, subdevices: Vec<SubDevice>) -> Self {
        Self {
            id,
            device_name,
            subdevices,
        }
    }
}

#[derive(Debug)]
pub struct SubDevice {
    id: u16,
    subvendor: u16,
    subdevice_name: String,
}

impl SubDevice {
    pub fn new(subvendor: u16, id: u16, subdevice_name: String) -> Self {
        Self {
            id,
            subvendor,
            subdevice_name,
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
