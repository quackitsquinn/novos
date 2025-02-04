use super::{
    class::PciDeviceClass,
    pci_read_u32,
    vendor_device::{get_vendor, Device, Vendor},
};

#[derive(Debug)]
pub struct PCIDevice {
    pub bus: u8,
    pub slot: u8,
    pub vendor: &'static Vendor,
    pub device: &'static Device,
    pub device_class: PciDeviceClass,
}

pub(super) fn pci_get_device(bus: u8, slot: u8) -> Option<PCIDevice> {
    let vid_did = pci_read_u32(bus, slot, 0, 0);
    if (vid_did & 0xFFFF) == 0xFFFF {
        return None;
    }

    let vendor_id = (vid_did & 0xFFFF) as u16;
    let device_id = (vid_did >> 16) as u16;
    let class_subclass = pci_read_u32(bus, slot, 0, 8);
    let class = (class_subclass >> 24) as u8;
    let subclass = (class_subclass >> 16) as u8;
    let prog_if = (class_subclass >> 8) as u8;

    let vendor = get_vendor(vendor_id)?;
    let device = vendor.get_device(device_id)?;

    let device_class = PciDeviceClass::new(class, subclass, prog_if).ok()?;

    Some(PCIDevice {
        bus,
        slot,
        vendor,
        device,
        device_class,
    })
}
