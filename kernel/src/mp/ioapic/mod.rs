//! Various Rust abstractions for IOAPIC support.
use core::fmt::Debug;

use ::acpi::sdt::madt::{Madt, MadtEntry};
use cake::log::info;
use cake::{Once, OnceMutex};
use modular_bitfield::prelude::*;

use crate::{
    acpi,
    memory::paging::phys::phys_mem::{self, PhysicalMemoryMap},
    mp::{apic_page_flags, id},
};

mod redirection;
mod version;

pub use redirection::RedirectionEntry;
pub use version::IoApicVersion;

/// Represents an I/O Advanced Programmable Interrupt Controller (IOAPIC).
#[derive(Debug)]
pub struct IoApic {
    base: Once<u64>,
    table: Once<PhysicalMemoryMap>,
    mapped: OnceMutex<*mut u8>,
}

impl IoApic {
    /// Creates an empty IOAPIC instance.
    pub const fn new() -> Self {
        Self {
            base: Once::new(),
            mapped: OnceMutex::uninitialized(),
            table: Once::new(),
        }
    }

    /// Initializes the IOAPIC by reading the MADT and mapping the IOAPIC's physical address into the kernel's address space.
    pub fn init(&self) {
        let madt = acpi::get_table::<Madt>().expect("Failed to get MADT");
        for entry in madt.table_pin().entries() {
            if let MadtEntry::IoApic(i) = entry {
                self.base.call_once(|| i.io_apic_address as u64);
                break;
            }
        }

        let base = *self.base.get().expect("No IOAPIC found in MADT");
        info!("IO APIC base address: {:#x}", base);
        let phys_addr = x86_64::PhysAddr::new(base);
        let map = unsafe {
            phys_mem::map_address(phys_addr, 1, apic_page_flags()).expect("Failed to map IOAPIC")
        };

        info!("Mapped IO APIC at {:p}", map.ptr());

        self.mapped.call_init(|| map.ptr().cast_mut());
        self.table.call_once(|| map);
    }

    /// Reads a value from the IOAPIC register at the given offset.
    ///
    /// # Safety
    /// The caller must ensure that the given offset is valid and that reading from the given offset will not cause undefined behavior.
    pub unsafe fn read_offset<T>(&self, byte_off: usize) -> T
    where
        T: Copy,
    {
        let ptr = self.mapped.get();
        let ptr = unsafe { ptr.add(byte_off) } as *const T;
        unsafe { ptr.read_volatile() }
    }

    /// Writes a value into the IOAPIC register at the given offset.
    ///
    /// # Safety
    /// The caller must ensure that the given offset is valid and that writing into the given offset will not cause undefined behavior.
    pub unsafe fn write_offset<T>(&self, byte_off: usize, value: T)
    where
        T: Copy,
    {
        let ptr = self.mapped.get();
        let ptr = unsafe { ptr.add(byte_off) } as *mut T;
        unsafe { ptr.write_volatile(value) }
    }

    /// Reads from the IOAPIC register specified by `reg`.
    pub unsafe fn read_register(&self, reg: u8) -> u32 {
        unsafe {
            self.write_offset(0x00, reg);
            self.read_offset(0x10)
        }
    }

    // Reads a 64-bit value from the IOAPIC by reading two consecutive 32-bit registers.
    /// # Safety
    /// The caller must ensure that reads are valid for `reg` and `reg + 1`.
    pub unsafe fn read_register_64(&self, reg: u8) -> u64 {
        unsafe {
            let low = self.read_register(reg) as u64;
            let high = self.read_register(reg + 1) as u64;
            (high << 32) | low
        }
    }

    /// Writes to the IOAPIC register specified by `reg`.
    /// # Safety
    /// The caller must ensure that the given register is valid.
    pub unsafe fn write_register(&self, reg: u8, value: u32) {
        unsafe { self.write_offset(0x00, reg) };
        unsafe { self.write_offset(0x10, value) };
    }

    /// Writes a 64-bit value to the IOAPIC by writing to two consecutive 32-bit registers.
    /// # Safety
    /// The caller must ensure that writes are valid for `reg` and `reg + 1`.
    pub fn write_register_64(&self, reg: u8, value: u64) {
        unsafe {
            self.write_register(reg, value as u32);
            self.write_register(reg + 1, (value >> 32) as u32);
        }
    }

    /// Reads the IOAPIC version register.
    pub fn version(&self) -> IoApicVersion {
        IoApicVersion::from_bytes(unsafe {
            self.read_register(IoApicVersion::REGISTER).to_ne_bytes()
        })
    }

    /// Reads the IOAPIC ID register.
    pub fn id(&self) -> IoApicId {
        IoApicId::from_bytes(unsafe { self.read_register(IoApicId::REGISTER as u8).to_ne_bytes() })
    }

    /// Updates the IOAPIC ID register.
    /// # Safety
    /// The caller must ensure that the new ID is valid and does not conflict with other IOAPIC IDs in the system.
    pub unsafe fn update_id(&self, new: IoApicId) {
        unsafe {
            self.write_register(
                IoApicId::REGISTER as u8,
                u32::from_ne_bytes(new.into_bytes()),
            )
        };
    }

    /// Reads the redirection entry at the given index.
    /// # Safety
    /// The caller must ensure that the given index is valid and does not exceed the maximum number
    /// of redirection entries supported by the IOAPIC.
    pub unsafe fn read_redirection_entry(&self, index: u8) -> RedirectionEntry {
        let val = unsafe { self.read_register_64(0x10 + (index * 2)) };
        RedirectionEntry::from_bytes(val.to_ne_bytes())
    }
}

/// Represents the ID register of the IOAPIC.
#[bitfield(bytes = 4)]
pub struct IoApicId {
    #[skip]
    __: B24,
    /// The ID of the IOAPIC.
    pub id: B4,
    #[skip]
    __: B4,
}

id!(IoApicId, REGISTER, 0x00);

impl Debug for IoApicId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IoApicId").field("id", &self.id()).finish()
    }
}
