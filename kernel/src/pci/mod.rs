use alloc::vec::Vec;
use device::{pci_get_device, PCIDevice};
use cake::spin::Mutex;
use x86_64::instructions::port::Port;

use crate::declare_module;

mod class;
mod device;
mod vendor_device;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PCIInitError {}

pub static PCI_DEVICES: Mutex<Vec<PCIDevice>> = Mutex::new(Vec::new());

fn init() -> Result<(), PCIInitError> {
    let mut pci_devices = PCI_DEVICES.lock();
    for i in 0..255 {
        for j in 0..32 {
            if let Some(device) = pci_get_device(i, j) {
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
