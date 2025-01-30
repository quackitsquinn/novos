use kproc::pci_ids;

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

pub fn get_device(vendor_id: u16, device_id: u16) -> Option<&'static Device> {
    for vendor in VENDORS {
        if vendor.id == vendor_id {
            for device in vendor.devices {
                if device.id == device_id {
                    return Some(device);
                }
            }
            return None;
        }
    }
    None
}

pci_ids!();
