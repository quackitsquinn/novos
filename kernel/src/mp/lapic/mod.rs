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

use crate::mp::lapic::timer::{ApicTimerLvt, TimerDivider};
use crate::{
    memory::paging::phys::phys_mem::{self, PhysicalMemoryMap},
    mp::apic_page_flags,
};

mod icr;
mod svr;
pub mod timer;
mod version;

pub use icr::{DeliverMode, DestinationShorthand, InterruptCommandRegister};
pub use svr::SpuriousInterruptVector;
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

        let translated = ACTIVE_PAGE_TABLE
            .read()
            .translate(VirtAddr::new(map.ptr() as u64));

        info!("LAPIC mapped at virtual address: {:#x?}", translated);
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

    /// Reads the Spurious Interrupt Vector Register (SVR).
    pub fn read_svr(&self) -> SpuriousInterruptVector {
        SpuriousInterruptVector(unsafe {
            self.read_offset::<u32>(SpuriousInterruptVector::REGISTER)
        })
    }

    /// Writes to the Spurious Interrupt Vector Register (SVR).
    ///
    /// # Safety
    /// The caller must also ensure that the value being written is valid.
    pub unsafe fn write_svr(&self, svr: SpuriousInterruptVector) {
        unsafe {
            self.write_offset::<u32>(SpuriousInterruptVector::REGISTER, svr.0);
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
            let low = self.read_offset::<u32>(InterruptCommandRegister::REGISTER);
            let high = self.read_offset::<u32>(InterruptCommandRegister::REGISTER + 0x10);
            InterruptCommandRegister(u64::from(high) << 32 | u64::from(low))
        }
    }

    /// Writes to the Interrupt Command Register (ICR).
    ///
    /// # Safety
    /// The caller must ensure that the given ICR value is valid.
    /// The caller must also ensure that the deliver status is not modified.
    pub unsafe fn write_icr(&self, icr: InterruptCommandRegister) {
        let icr: u64 = icr.0;
        trace!("Writing ICR: {:#016x}", icr);
        let low: u32 = (icr & 0xFFFF_FFFF) as u32;
        let high: u32 = (icr >> 32) as u32;
        unsafe {
            self.write_offset::<u32>(InterruptCommandRegister::REGISTER + 0x10, high);
            self.write_offset::<u32>(InterruptCommandRegister::REGISTER, low);
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

    /// Reads the Local Vector Table (LVT) Timer Register.
    pub fn read_lvt_timer(&self) -> ApicTimerLvt {
        unsafe { ApicTimerLvt(self.read_offset::<u32>(ApicTimerLvt::REGISTER)) }
    }

    /// Writes to the Local Vector Table (LVT) Timer Register.
    /// # Safety
    /// The caller must ensure that the given LVT Timer value is valid and does not conflict with other LVT entries.
    pub unsafe fn write_lvt_timer(&self, lvt: ApicTimerLvt) {
        unsafe {
            self.write_offset::<u32>(ApicTimerLvt::REGISTER, lvt.0);
        }
    }

    /// Updates the Local Vector Table (LVT) Timer Register by applying the given function to the current value.
    /// # Safety
    /// The caller must ensure that the given LVT Timer closure will keep the LVT entry valid and does not conflict with other LVT entries.
    pub unsafe fn update_lvt_timer<F>(&self, f: F)
    where
        F: FnOnce(&mut ApicTimerLvt),
    {
        let mut lvt = self.read_lvt_timer();
        f(&mut lvt);
        unsafe { self.write_lvt_timer(lvt) };
    }

    /// Sets the LAPIC timer divider.
    pub fn set_timer_divider(&self, divider: TimerDivider) {
        let val = divider as u8;
        // For some reason, there is a reserved bit in the middle of the divider value. Intel loves making things complicated.
        let up_bit = (val << 1) & 0b1000;
        let low_bits = val & 0b11;
        let final_val = up_bit | low_bits;
        trace!("Setting LAPIC timer divider to: {:#05b}", final_val);
        unsafe {
            self.write_offset::<u32>(LAPIC_TIMER_DIVIDE_OFFSET, final_val as u32);
        }
    }

    /// Reads the current count value of the LAPIC timer.
    pub fn read_timer_current_count(&self) -> u32 {
        unsafe { self.read_offset::<u32>(LAPIC_TIMER_CURRENT_COUNT_OFFSET) }
    }

    /// Writes the initial count value to the LAPIC timer.
    pub fn write_timer_initial_count(&self, count: u32) {
        unsafe {
            self.write_offset::<u32>(LAPIC_TIMER_INITIAL_COUNT_OFFSET, count);
        }
    }

    /// Attempts to read the base frequency of the LAPIC timer in Hz.
    /// Returns `None` if the frequency cannot be determined.
    pub fn timer_base_freq_hz(&self) -> Option<u64> {
        raw_cpuid::CpuId::with_cpuid_reader(raw_cpuid::CpuIdReaderNative)
            .get_tsc_info()
            .map_or(None, |tsc_info| tsc_info.tsc_frequency())
    }

    /// Checks if the LAPIC timer is of consistent speed (i.e., not affected by power-saving modes).
    /// Returns `true` if the timer is consistent speed, `false` otherwise.
    #[doc(alias = "arat")]
    pub fn timer_is_consistent_speed(&self) -> bool {
        raw_cpuid::CpuId::with_cpuid_reader(raw_cpuid::CpuIdReaderNative)
            .get_thermal_power_info()
            .map_or(false, |therm_info| therm_info.has_arat())
    }
}

/// SAFETY: The LAPIC is safe to access from multiple threads, as long as the caller ensures that
/// the LAPIC has been properly initialized before use.
unsafe impl Sync for Lapic {}
unsafe impl Send for Lapic {}
