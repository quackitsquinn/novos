//! Various rust abstractions for Local APIC (LAPIC) management.
//!
//! Most of the documentation for individual types are taken directly from section 3A of
//! the Intel® 64 and IA-32 Architectures Software Developer’s Manual
use cake::Once;
use cake::limine::mp::Cpu;
use cake::log::{info, trace};
use x86_64::VirtAddr;
use x86_64::registers::model_specific::Msr;
use x86_64::structures::paging::Translate;

use crate::memory::paging::ACTIVE_PAGE_TABLE;

use crate::mp::lapic::icr::InterruptCommandRegister;
use crate::mp::lapic::svr::SpuriousInterruptVector;

use crate::{
    memory::paging::phys::phys_mem::{self, PhysicalMemoryMap},
    mp::apic_page_flags,
};

pub mod icr;
pub mod lvt;
pub mod svr;

mod version;

pub use svr::SpuriousInterruptVectorValue;
pub use version::LapicVersion;

/// The Model Specific Register (MSR) used to determine the base address of the Local APIC.
pub const LAPIC_BASE_MSR: Msr = Msr::new(0x1B);

/// The offset for the End Of Interrupt (EOI) register.
pub const LAPIC_EOI_OFFSET: usize = 0xB0;
/// The offset for the Timer Divide Configuration register of the LAPIC timer.
pub const LAPIC_TIMER_DIVIDE_OFFSET: usize = 0x3E0;
/// The offset for the Initial Count register of the LAPIC timer.
pub const LAPIC_TIMER_INITIAL_COUNT_OFFSET: usize = 0x380;
/// The offset for the Current Count register of the LAPIC timer.
pub const LAPIC_TIMER_CURRENT_COUNT_OFFSET: usize = 0x390;

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
        let map = unsafe {
            phys_mem::map_address(phys_addr, 1, apic_page_flags()).expect("Failed to map LAPIC")
        };

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
        LapicVersion(unsafe { self.read_offset::<u32>(LapicVersion::REGISTER) })
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

    /// Provides access to the Spurious Interrupt Vector Register (SVR) interface.
    pub fn spurious_interrupt_vector(&self) -> SpuriousInterruptVector<'_> {
        SpuriousInterruptVector::new(self)
    }

    /// Provides access to the Interrupt Command Register (ICR) interface.
    pub fn icr(&self) -> InterruptCommandRegister<'_> {
        InterruptCommandRegister::new(self)
    }

    /// Provides access to the Local Vector Table (LVT) interface.
    pub fn lvt(&self) -> lvt::LocalVectorTable<'_> {
        lvt::LocalVectorTable::new(self)
    }
}

/// SAFETY: The LAPIC is safe to access from multiple threads, as long as the caller ensures that
/// the LAPIC has been properly initialized before use.
unsafe impl Sync for Lapic {}
unsafe impl Send for Lapic {}
