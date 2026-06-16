use crate::{
    MapFlags, arch,
    paging::{Address, Frame, PhysAddr, PrimitiveSize},
};

/// A page table, accurate to the current architecture.
#[repr(C)]
#[cfg_attr(feature = "x86_64", repr(align(4096)))]
#[derive(Debug)]
pub struct PageTable {
    entries: [PageTableEntry; arch::ENTRY_COUNT],
}

impl PageTable {
    /// Creates a new empty page table with all entries set to zero.
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry { value: 0 }; arch::ENTRY_COUNT],
        }
    }

    /// Clears the page table by setting all entries to zero.
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.value = 0;
        }
    }

    /// Returns a reference to the entries of this page table.
    pub fn entries(&self) -> &[PageTableEntry; arch::ENTRY_COUNT] {
        &self.entries
    }

    /// Returns a mutable reference to the entries of this page table.
    ///
    /// # Safety
    /// The caller must ensure that any modifications to the entries do not violate memory safety, e.g. by writing invalid values or creating invalid mappings.
    pub unsafe fn entries_mut(&mut self) -> &mut [PageTableEntry; arch::ENTRY_COUNT] {
        &mut self.entries
    }
}

/// A page table entry, representing a single entry in a page table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PageTableEntry {
    value: u64,
}

impl PageTableEntry {
    /// Creates a new page table entry with the given physical frame and flags.
    pub fn new<S: PrimitiveSize>(phys: Frame<S>, flags: arch::ArchEntryFlags) -> Self {
        let addr = phys.start_address().as_u64();
        Self {
            value: addr | flags.bits(),
        }
    }

    /// The arch specific flags of this page table entry, as a `arch::ArchEntryFlags` bitflags struct.
    pub fn arch_flags(&self) -> arch::ArchEntryFlags {
        arch::ArchEntryFlags::from_bits_truncate(self.value)
    }

    /// The flags of this page table entry, as a `MapFlags` bitflags struct.
    pub fn flags(&self) -> MapFlags {
        arch::ArchEntryFlags::from_bits_truncate(self.value).into()
    }

    /// Sets the flags of this page table entry to the given `MapFlags`, while preserving the address bits.
    pub fn set_flags(&mut self, flags: MapFlags) {
        let arch_flags: arch::ArchEntryFlags = flags.into();
        self.value = (self.value & !arch_flags.bits()) | arch_flags.bits();
    }

    /// Returns the physical address contained in this page table entry, if it is present and valid.
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.value & arch::PHYSICAL_ADDRESS_MAX)
    }
}
