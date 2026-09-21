use core::fmt::{Debug, Display};

use crate::{
    MapFlags, arch,
    paging::{
        Address, AddressExt, FragmentSize, Frame, MemoryFragment, Page, PageTableIndex, PhysAddr,
        Small, VirtAddr,
    },
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

    /// Returns the page that contains this page table.
    pub fn as_page(&self) -> Page<Small> {
        Page::from_start_address(VirtAddr::from_ptr(self).unwrap()).unwrap()
    }

    /// Updates the given entry in the page table with the provided `PageTableEntry`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the index is valid and that the entry being set does not
    /// violate memory safety, e.g., by creating invalid mappings or overwriting critical entries.
    pub unsafe fn set_entry(&mut self, index: PageTableIndex, entry: PageTableEntry) {
        self.entries[index.value() as usize] = entry;
    }

    /// Reads the entry at the given index in the page table.
    pub fn read_entry(&self, index: PageTableIndex) -> PageTableEntry {
        self.entries[index.value() as usize]
    }

    /// Returns the virtual address of this page table.
    pub fn as_virt(&self) -> VirtAddr {
        VirtAddr::from_ptr(self).unwrap()
    }

    /// Returns whether any entry in this page table is present (i.e., valid and mapped).
    pub fn is_empty(&self) -> bool {
        self.entries.iter().any(|entry| entry.is_present())
    }

    /// Zeros out all entries in the page table, effectively clearing it.
    pub unsafe fn zero(&mut self) {
        for entry in unsafe { self.entries_mut() }.iter_mut() {
            *entry = PageTableEntry::empty();
        }
    }
}

/// A page table entry, representing a single entry in a page table.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PageTableEntry {
    value: u64,
}

impl PageTableEntry {
    /// Creates a new page table entry with the given physical frame and flags.
    pub fn new<S: FragmentSize>(phys: Frame<S>, flags: MapFlags) -> Self {
        let addr = phys.start_address().as_u64();
        Self {
            value: addr | arch::ArchEntryFlags::from(flags).bits(),
        }
    }

    /// Creates an empty page table entry with all bits set to zero.
    pub fn empty() -> Self {
        Self { value: 0 }
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
        PhysAddr::new(self.value & arch::PAGE_TABLE_ENTRY_ADDR_BITS)
    }

    /// Returns whether this page table entry is present (i.e., valid and mapped).
    pub fn is_present(&self) -> bool {
        self.arch_flags().contains(arch::ArchEntryFlags::PRESENT)
    }

    /// Returns whether this page table entry is marked as huge (i.e., representing a large page).
    pub fn is_huge(&self) -> bool {
        self.arch_flags().contains(arch::ArchEntryFlags::HUGE_PAGE)
    }

    /// Sets or clears the huge page flag for this page table entry.
    pub fn huge(mut self) -> Self {
        self.value |= arch::ArchEntryFlags::HUGE_PAGE.bits();
        self
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("addr", &format_args!("{:?}", self.addr()))
            .field("flags", &self.flags())
            .finish()
    }
}

impl Display for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "pte(addr: {:#x}, flags: {})",
            self.addr().as_u64(),
            self.flags()
        )
    }
}
