use kproc::pci_ids;
use log::info;

// Ok. It's scheming time.
// My plan is a (hopefullY) small proc macro that will generate the PCI device structs for us using pci.ids as the source.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Vendor {
    pub id: u16,
    pub name: &'static str,
    pub devices: &'static [Device],
}

impl Vendor {
    pub const fn new(id: u16, name: &'static str, devices: &'static [Device]) -> Self {
        Self { id, name, devices }
    }

    pub fn get_device(&self, device_id: u16) -> Option<&'static Device> {
        for device in self.devices {
            if device.id == device_id {
                return Some(device);
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Device {
    pub id: u16,
    pub name: &'static str,
    pub devices: &'static [SubDevice],
}

impl Device {
    pub const fn new(id: u16, name: &'static str, devices: &'static [SubDevice]) -> Self {
        Self { id, name, devices }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct SubDevice {
    pub subvendor: u16,
    pub id: u16,
    pub name: &'static str,
}

impl SubDevice {
    pub const fn new(subvendor: u16, id: u16, name: &'static str) -> Self {
        Self {
            subvendor,
            id,
            name,
        }
    }
}
// SubDevice::New
// Shortcut to reduce size of pci_ids
const fn sdn(subvendor: u16, id: u16, name: &'static str) -> SubDevice {
    SubDevice::new(subvendor, id, name)
}
// Device::New
// Shortcut to reduce size of pci_ids
const fn dn(id: u16, name: &'static str, devices: &'static [SubDevice]) -> Device {
    Device::new(id, name, devices)
}

// Vendor::New
// Shortcut to reduce size of pci_ids
const fn vn(id: u16, name: &'static str, devices: &'static [Device]) -> Vendor {
    Vendor::new(id, name, devices)
}

pub fn get_vendor(vendor_id: u16) -> Option<&'static Vendor> {
    //info!("Vendor count: {}", VENDORS.len());
    // for vendor in VENDORS {
    //     if vendor.id == vendor_id {
    //         return Some(vendor);
    //     }
    // }
    None
}
// TODO: This solution is not ideal. rust-analyzer is not happy with the 50k~ lines of code that pci_ids generates.
// Even using the shorthands, it's still 2.1mb of code. Im thinking of just creating a binary file that will be embedded in the kernel binary.
