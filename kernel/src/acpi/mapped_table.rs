//! Support for mapping dynamically sized ACPI tables from physical memory.
use core::{
    fmt,
    pin::{Pin, pin},
};

use acpi::{AcpiError, AcpiTable, sdt::SdtHeader};
use cake::Owned;
use nmm::{
    MapFlags, MemoryMapping,
    paging::{AddressExt, VirtAddr},
};
use x86_64::{PhysAddr, structures::paging::PageTableFlags};

/// A mapped ACPI table.
pub struct MappedTable<T: AcpiTable> {
    table: Owned<T>,
    sdt: Owned<SdtHeader>,
    mapping: MemoryMapping,
}

impl<T: AcpiTable> MappedTable<T> {
    /// Creates a new mapped ACPI table from the given physical address.
    /// # Safety
    /// The caller must ensure that the physical address is valid and that the table is not already mapped.
    pub unsafe fn new(addr: PhysAddr) -> Result<Self, AcpiError> {
        let mut table_mapping =
            nmm::create_phys_mapping(addr.into(), core::mem::size_of::<T>(), MapFlags::WRITABLE)
                .expect("Failed to map ACPI table");

        let table: &mut SdtHeader = unsafe { &mut *(table_mapping.as_mut_ptr()) };

        if table.signature != T::SIGNATURE {
            return Err(AcpiError::SdtInvalidSignature(table.signature));
        }

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = nmm::create_phys_mapping(addr.into(), length, MapFlags::WRITABLE)
                .expect("Failed to map full ACPI table");
            unsafe { nmm::free_phys_mapping(table_mapping) };
            table_mapping = new_phys_map;
        }

        // FIXME: I don't know how I never noticed this, but this is flagrantly undefined behavior.

        let table: Owned<T> = unsafe { Owned::new(table_mapping.as_mut_ptr()) };

        table.validate()?;

        let sdt = unsafe { Owned::new(table_mapping.as_mut_ptr()) };

        Ok(Self {
            table,
            sdt,
            mapping: table_mapping,
        })
    }

    /// Creates a new mapped ACPI table from the given physical address without validating the signature or the checksum.
    ///
    /// # Safety
    /// The caller must ensure that the given physical address contains a valid ACPI table of type `T`.
    pub unsafe fn new_unchecked(addr: PhysAddr) -> Self {
        let mut mapping =
            nmm::create_phys_mapping(addr.into(), core::mem::size_of::<T>(), MapFlags::WRITABLE)
                .expect("Failed to map ACPI table");

        let table: &mut SdtHeader = unsafe { &mut *(mapping.as_mut_ptr()) };

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = nmm::create_phys_mapping(addr.into(), length, MapFlags::WRITABLE)
                .expect("Failed to map full ACPI table");
            unsafe { nmm::free_phys_mapping(mapping) };
            mapping = new_phys_map;
        }

        // FIXME: Again, flagrantly undefined behavior. I don't know how I never noticed this, but here we are.
        // speaking of..
        // TODO: Owned -> Unique. Owned doesn't stress the uniqueness of the pointer, but really it should.
        let table: Owned<T> = unsafe { Owned::new(mapping.as_mut_ptr()) };

        let sdt = unsafe { Owned::new(mapping.as_mut_ptr()) };

        Self {
            table,
            sdt,
            mapping,
        }
    }

    /// Returns a reference to the mapped ACPI table.
    pub fn table(&self) -> &T {
        &self.table
    }

    /// Returns a pinned reference to the mapped ACPI table.
    pub fn table_pin(&self) -> Pin<&T> {
        // SAFETY: The backing table is never moved or re-mapped while this MappedTable exists,
        // so the pointer is stable and valid for the lifetime of this MappedTable
        unsafe { Pin::new_unchecked(&*self.table) }
    }
}

impl<T: AcpiTable> Drop for MappedTable<T> {
    fn drop(&mut self) {
        unsafe { nmm::free_phys_mapping(self.mapping) };
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
            .finish()
    }
}
