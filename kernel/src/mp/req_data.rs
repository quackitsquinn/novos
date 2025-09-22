use core::ops::Deref;

use arrayvec::ArrayVec;

use cake::{
    limine::{mp::Cpu, response::MpResponse},
    LimineData,
};

/// A single application core, with its APIC ID and LAPIC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationCore {
    pub apic_id: u32,
    pub lapic: u32,
}

impl ApplicationCore {
    /// Creates a new `ApplicationCore` with the given APIC ID and LAPIC address.
    pub const fn new(cpu: &Cpu) -> Self {
        let apic_id = cpu.id;
        let lapic = cpu.lapic_id; // TODO: Replace with actual LAPIC

        Self { apic_id, lapic }
    }
}

impl From<&Cpu> for ApplicationCore {
    fn from(cpu: &Cpu) -> Self {
        Self::new(cpu)
    }
}

/// A collection of application cores, along with the original Limine data for advanced use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationCores {
    cores: ArrayVec<ApplicationCore, 256>,
}

impl ApplicationCores {
    /// Creates a new `ApplicationCores` from the given Limine MP response. Mainly intended for internal use.
    pub fn new(response: LimineData<MpResponse>) -> Self {
        Self {
            // SAFETY: The lifetime of the limine data is 'static as long as the requests are not terminated.
            cores: response
                .cpus()
                .iter()
                .map(|cpu| ApplicationCore::from(*cpu))
                .collect(),
        }
    }

    /// Returns a slice of the application cores.
    pub fn get(&self) -> &[ApplicationCore] {
        &self.cores
    }
}

impl Deref for ApplicationCores {
    type Target = [ApplicationCore];

    fn deref(&self) -> &Self::Target {
        &self.cores
    }
}

// TODO: Is this actually safe?
unsafe impl Send for ApplicationCores {}
unsafe impl Sync for ApplicationCores {}
