use alloc::vec::Vec;
use class::PciDeviceClass;
use spin::Mutex;
use x86_64::{addr, instructions::port::Port};

use crate::declare_module;

mod class;
mod vendor_device;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PCIInitError {}

pub static PCI_DEVICES: Mutex<Vec<PCIDevice>> = Mutex::new(Vec::new());

fn init() -> Result<(), PCIInitError> {
    let mut pci_devices = PCI_DEVICES.lock();
    for i in 0..255 {
        for j in 0..32 {
            if let Some(device) = pci_get_device(i, j) {
                if let Ok(class_tree) =
                    PciDeviceClass::new(device.class, device.subclass, device.prog_if)
                {
                    if let Some(device_vendor_info) =
                        vendor_device::get_device(device.vendor_id, device.device_id)
                    {
                        log::info!(
                            "Found PCI device: {:02x}:{:02x} - {:04x}:{:04x} - {:?}: {}",
                            device.bus,
                            device.slot,
                            device.vendor_id,
                            device.device_id,
                            class_tree,
                            device_vendor_info.name
                        );
                    } else {
                        log::info!(
                            "Found PCI device: {:02x}:{:02x} - {:04x}:{:04x} - {:?}: UNKNOWN",
                            device.bus,
                            device.slot,
                            device.vendor_id,
                            device.device_id,
                            class_tree
                        );
                    }
                } else {
                    // This will probably literally never run
                    log::info!(
                        "Unidentified PCI device found: {:02x}:{:02x} - {:04x}:{:04x} - {:02x}:{:02x}:{:02x}",
                        device.bus,
                        device.slot,
                        device.vendor_id,
                        device.device_id,
                        device.class,
                        device.subclass,
                        device.prog_if,
                    );
                }
                pci_devices.push(device);
            }
        }
    }
    Ok(())
}

declare_module!("PCI", init, PCIInitError);

static PCI_COMMS: Mutex<(Port<u32>, Port<u32>)> = Mutex::new((Port::new(0xCF8), Port::new(0xCFC)));

fn pci_read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let mut comms = PCI_COMMS.lock();
    let (cfg, data) = &mut *comms;

    let bus = (bus as u32) << 16;
    let slot = (slot as u32) << 11;
    let func = (func as u32) << 8;
    let offset = (offset as u32) & 0xFC;

    let mut address: u32 = 0x8000_0000;

    address |= bus | slot | func | offset;

    unsafe {
        cfg.write(address);
        data.read()
    }
}

#[derive(Debug)]
struct PCIDevice {
    bus: u8,
    slot: u8,
    vendor_id: u16,
    device_id: u16,
    class: u8,
    subclass: u8,
    prog_if: u8,
}

fn pci_get_device(bus: u8, slot: u8) -> Option<PCIDevice> {
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

    Some(PCIDevice {
        bus,
        slot,
        vendor_id: vendor_id,
        device_id,
        class,
        subclass,
        prog_if,
    })
}
