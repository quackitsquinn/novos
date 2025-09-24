use cake::spin::Once;
use log::{debug, info};
use x86_64::{registers::model_specific::Msr, structures::paging::PageTableFlags};

use crate::{
    memory::paging::phys::phys_mem::{self, PhysicalMemoryMap},
    mp::apic_page_flags,
};

mod icr;
mod svr;
mod version;

pub use icr::{DeliverMode, DestinationShorthand, InterruptCommandRegister};
pub use svr::SpuriousInterruptVector;
pub use version::LapicVersion;

pub const LAPIC_BASE_MSR: Msr = Msr::new(0x1B);

pub const LAPIC_VERSION_OFFSET: usize = 0x30;
pub const LAPIC_EOI_OFFSET: usize = 0xB0;
pub const LAPIC_SVR_OFFSET: usize = 0xF0;
pub const LAPIC_ICR_OFFSET: usize = 0x300;
pub const LAPIC_LVT_TIMER_OFFSET: usize = 0x320;

/// Represents the Local APIC (LAPIC) of the CPU.
/// Provides methods to read and write LAPIC registers, send interrupts, and manage LAPIC state
/// such as enabling/disabling the LAPIC and handling End Of Interrupt (EOI) signals.
#[derive(Debug)]
pub struct Lapic {
    base: Once<u64>,
    table: Once<PhysicalMemoryMap>,
    mapped: Once<*mut u8>,
}

impl Lapic {
    /// Creates a new, uninitialized LAPIC instance.
    pub const fn new() -> Self {
        Self {
            base: Once::new(),
            mapped: Once::new(),
            table: Once::new(),
        }
    }

    /// Initializes the LAPIC by reading the LAPIC base address from the MSR and mapping it into the kernel's address space.
    /// This function must be called before any other LAPIC functions are used.
    pub fn init(&self) {
        let base = unsafe { LAPIC_BASE_MSR.read() } & 0xFFFF_FFFF_FFFF_F000;
        self.base.call_once(|| base);
        info!("LAPIC base address: {:#x}", base);
        let phys_addr = x86_64::PhysAddr::new(base);
        let map =
            phys_mem::map_address(phys_addr, 1, apic_page_flags()).expect("Failed to map LAPIC");

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
        LapicVersion::from_bytes(unsafe {
            self.read_offset::<u32>(LAPIC_VERSION_OFFSET).to_ne_bytes()
        })
    }

    /// Sends an End Of Interrupt (EOI) signal to the LAPIC.
    /// # Safety
    /// The caller must ensure that the LAPIC has been properly initialized.
    /// The caller must also ensure that this is called in response to an interrupt.
    pub unsafe fn eoi(&self) {
        unsafe {
            self.write_offset::<u32>(LAPIC_EOI_OFFSET, 0);
        }
    }

    /// Reads the Spurious Interrupt Vector Register (SVR).
    pub fn read_svr(&self) -> SpuriousInterruptVector {
        SpuriousInterruptVector::from_bytes(unsafe {
            self.read_offset::<u32>(LAPIC_SVR_OFFSET).to_ne_bytes()
        })
    }

    /// Writes to the Spurious Interrupt Vector Register (SVR).
    ///
    /// # Safety
    /// The caller must also ensure that the value being written is valid.
    pub unsafe fn write_svr(&self, svr: SpuriousInterruptVector) {
        unsafe {
            self.write_offset::<u32>(LAPIC_SVR_OFFSET, u32::from_ne_bytes(svr.into_bytes()));
        }
    }

    /// Updates the Spurious Interrupt Vector Register (SVR) by applying the given function to the current value.
    /// # Safety
    /// The caller must ensure that the given function does not violate any invariants of the SVR.
    pub unsafe fn update_svr<F>(&self, f: F)
    where
        F: FnOnce(&mut SpuriousInterruptVector),
    {
        let mut svr = self.read_svr();
        f(&mut svr);
        unsafe { self.write_svr(svr) };
    }

    /// Enables the LAPIC by setting the APIC enable bit in the SVR and setting the spurious interrupt vector.
    /// # Safety
    /// The caller must ensure that the LAPIC is in a valid state to be enabled.
    /// The caller must also ensure that the current IDT is valid for the current LAPIC configuration.
    pub unsafe fn enable(&self, vector: u8) {
        unsafe {
            self.update_svr(|svr| {
                svr.set_vector(vector);
                svr.set_apic_enable(true);
            });
        }
    }

    /// Reads the Interrupt Command Register (ICR).
    pub fn read_icr(&self) -> InterruptCommandRegister {
        unsafe {
            InterruptCommandRegister::from_bytes(self.read_offset::<[u8; 8]>(LAPIC_ICR_OFFSET))
        }
    }

    /// Writes to the Interrupt Command Register (ICR).
    ///
    /// # Safety
    /// The caller must ensure that the given ICR value is valid.
    /// The caller must also ensure that the deliver status is not modified.
    pub unsafe fn write_icr(&self, icr: InterruptCommandRegister) {
        unsafe {
            self.write_offset::<[u8; 8]>(LAPIC_ICR_OFFSET, icr.into_bytes());
        }
    }

    /// Updates the Interrupt Command Register (ICR) by applying the given function to the current value.
    /// # Safety
    /// The caller must ensure that the given function does not modify the deliver status bit.
    pub unsafe fn update_icr<F>(&self, f: F)
    where
        F: FnOnce(&mut InterruptCommandRegister),
    {
        let mut icr = self.read_icr();
        f(&mut icr);
        unsafe { self.write_icr(icr) };
    }
}

/// SAFETY: The LAPIC is safe to access from multiple threads, as long as the caller ensures that
/// the LAPIC has been properly initialized before use.
unsafe impl Sync for Lapic {}
unsafe impl Send for Lapic {}
