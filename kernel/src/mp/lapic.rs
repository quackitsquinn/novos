use core::fmt::Debug;

use cake::{spin::Once, OnceMutex};
use log::info;
use modular_bitfield::prelude::*;
use x86_64::{registers::model_specific::Msr, structures::paging::PageTableFlags};

use crate::memory::paging::phys::{
    self, mapper,
    phys_mem::{self, PhysicalMemoryMap},
};

pub const LAPIC_BASE_MSR: Msr = Msr::new(0x1B);

pub struct Lapic {
    base: Once<u64>,
    table: Once<PhysicalMemoryMap>,
    mapped: Once<*mut u8>,
}

impl Lapic {
    pub const fn new() -> Self {
        Self {
            base: Once::new(),
            mapped: Once::new(),
            table: Once::new(),
        }
    }

    pub fn init(&self) {
        let base = unsafe { LAPIC_BASE_MSR.read() } & 0xFFFF_FFFF_FFFF_F000;
        self.base.call_once(|| base);
        info!("LAPIC base address: {:#x}", base);
        let phys_addr = x86_64::PhysAddr::new(base);
        let map = phys_mem::map_address(
            phys_addr,
            1,
            PageTableFlags::PRESENT | PageTableFlags::NO_CACHE | PageTableFlags::WRITABLE,
        )
        .expect("Failed to map LAPIC");

        self.table.call_once(|| map);
        self.mapped.call_once(|| map.ptr().cast_mut());
    }

    fn base_ptr(&self) -> *mut u8 {
        *self.mapped.wait()
    }

    /// Reads a value from the LAPIC register at the given offset.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given offset is valid and that the LAPIC has been properly initialized.
    pub unsafe fn read_offset<T>(&self, byte_off: usize) -> T
    where
        T: Copy,
    {
        let ptr = unsafe { self.base_ptr().add(byte_off) } as *const T;
        unsafe { ptr.read_volatile() }
    }

    /// Writes a value to the LAPIC register at the given offset.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given offset is valid and that the LAPIC has been properly initialized.
    pub unsafe fn write_offset<T>(&self, byte_off: usize, value: T)
    where
        T: Copy,
    {
        let ptr = unsafe { self.base_ptr().add(byte_off) } as *mut T;
        unsafe { ptr.write_volatile(value) }
    }

    /// Reads the LAPIC version register.
    pub fn version(&self) -> LapicVersion {
        unsafe { self.read_offset(0x30) }
    }
}

#[derive(Clone, Copy)]
#[bitfield(bytes = 4)]
pub struct LapicVersion {
    version: u8,
    #[skip]
    __: B8,
    max_lvt_entry: B7,
    supports_eoi_broadcast_suppression: bool,
    #[skip]
    __: B8,
}

impl Debug for LapicVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LapicVersion")
            .field("version", &self.version())
            .field("max_lvt_entry", &self.max_lvt_entry())
            .field(
                "supports_eoi_broadcast_suppression",
                &self.supports_eoi_broadcast_suppression(),
            )
            .finish()
    }
}

/// SAFETY: The LAPIC is safe to access from multiple threads, as long as the caller ensures that
/// the LAPIC has been properly initialized before use.
unsafe impl Sync for Lapic {}
unsafe impl Send for Lapic {}
