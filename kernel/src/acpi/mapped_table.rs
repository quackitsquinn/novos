//! Support for mapping dynamically sized ACPI tables from physical memory.
use core::{
    fmt,
    pin::{Pin, pin},
};

use acpi::{AcpiError, AcpiTable, sdt::SdtHeader};
use cake::Owned;
use x86_64::{PhysAddr, structures::paging::PageTableFlags};

use crate::memory::paging::phys::phys_mem::{PhysicalMemoryMap, map_address, unmap_address};
/// A mapped ACPI table.
pub struct MappedTable<T: AcpiTable> {
    table: Owned<T>,
    sdt: Owned<SdtHeader>,
    phys_range: PhysicalMemoryMap,
}

impl<T: AcpiTable> MappedTable<T> {
    /// Creates a new mapped ACPI table from the given physical address.
    /// # Safety
    /// The caller must ensure that the physical address is valid and that the table is not already mapped.
    pub unsafe fn new(addr: PhysAddr) -> Result<Self, AcpiError> {
        let mut phys_map = unsafe {
            map_address(
                addr,
                core::mem::size_of::<T>() as u64,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            )
            .expect("Failed to map ACPI table")
        };

        let table: &mut SdtHeader = unsafe { &mut *(phys_map.ptr() as *mut SdtHeader) };

        if table.signature != T::SIGNATURE {
            return Err(AcpiError::SdtInvalidSignature(table.signature));
        }

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = unsafe {
                map_address(
                    addr,
                    length as u64,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                )
            }
            .expect("Failed to map full ACPI table");
            unmap_address(phys_map);
            phys_map = new_phys_map;
        }

        let table: Owned<T> = unsafe { Owned::new(phys_map.ptr().cast_mut().cast()) };

        table.validate()?;

        let sdt = unsafe { Owned::new(phys_map.ptr() as *mut SdtHeader) };

        Ok(Self {
            table,
            phys_range: phys_map,
            sdt,
        })
    }

    /// Creates a new mapped ACPI table from the given physical address without validating the signature or the checksum.
    ///
    /// # Safety
    /// The caller must ensure that the given physical address contains a valid ACPI table of type `T`.
    pub unsafe fn new_unchecked(addr: PhysAddr) -> Self {
        let mut phys_map = unsafe {
            map_address(
                addr,
                core::mem::size_of::<T>() as u64,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            )
            .expect("Failed to map ACPI table")
        };

        let table: &mut SdtHeader = unsafe { &mut *(phys_map.ptr() as *mut SdtHeader) };

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = unsafe {
                map_address(
                    addr,
                    length as u64,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                )
            }
            .expect("Failed to map full ACPI table");
            unmap_address(phys_map);
            phys_map = new_phys_map;
        }

        let table: Owned<T> = unsafe { Owned::new(phys_map.ptr().cast_mut().cast()) };

        let sdt = unsafe { Owned::new(phys_map.ptr() as *mut SdtHeader) };

        Self {
            table,
            phys_range: phys_map,
            sdt,
        }
    }

    /// Returns a reference to the mapped ACPI table.
    pub fn table(&self) -> &T {
        &self.table
    }

    /// Returns a pinned reference to the mapped ACPI table.
    pub fn table_pin(&self) -> Pin<&T> {
        pin!(&self.table)
    }
}

impl<T: AcpiTable> Drop for MappedTable<T> {
    fn drop(&mut self) {
        unmap_address(self.phys_range);
    }
}

impl<T: AcpiTable> core::ops::Deref for MappedTable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

impl<T: AcpiTable> core::ops::DerefMut for MappedTable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.table
    }
}

impl<T: AcpiTable> fmt::Debug for MappedTable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MappedTable")
            .field("sdt", &self.sdt)
            .field("phys_range", &self.phys_range)
            .finish()
    }
}
