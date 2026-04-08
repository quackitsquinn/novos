//! Support for mapping dynamically sized ACPI tables from physical memory.
use core::{
    fmt,
    pin::{Pin, pin},
};

use acpi::{AcpiError, AcpiTable, sdt::SdtHeader};
use cake::Owned;
use nmm::{MapFlags, arch::VirtAddr};
use x86_64::{PhysAddr, structures::paging::PageTableFlags};

/// A mapped ACPI table.
pub struct MappedTable<T: AcpiTable> {
    table: Owned<T>,
    sdt: Owned<SdtHeader>,
}

impl<T: AcpiTable> MappedTable<T> {
    /// Creates a new mapped ACPI table from the given physical address.
    /// # Safety
    /// The caller must ensure that the physical address is valid and that the table is not already mapped.
    pub unsafe fn new(addr: PhysAddr) -> Result<Self, AcpiError> {
        let mut table_addr =
            nmm::map_alloc(addr.into(), core::mem::size_of::<T>(), MapFlags::WRITABLE)
                .expect("Failed to map ACPI table");

        let table: &mut SdtHeader = unsafe { &mut *(table_addr.as_mut_ptr()) };

        if table.signature != T::SIGNATURE {
            return Err(AcpiError::SdtInvalidSignature(table.signature));
        }

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = nmm::map_alloc(addr.into(), length, MapFlags::WRITABLE)
                .expect("Failed to map full ACPI table");
            unsafe { nmm::unmap(table_addr.into(), core::mem::size_of::<T>()) };
            table_addr = new_phys_map;
        }

        // FIXME: I don't know how I never noticed this, but this is flagrantly undefined behavior.

        let table: Owned<T> = unsafe { Owned::new(table_addr.as_mut_ptr()) };

        table.validate()?;

        let sdt = unsafe { Owned::new(table_addr.as_mut_ptr()) };

        Ok(Self { table, sdt })
    }

    /// Creates a new mapped ACPI table from the given physical address without validating the signature or the checksum.
    ///
    /// # Safety
    /// The caller must ensure that the given physical address contains a valid ACPI table of type `T`.
    pub unsafe fn new_unchecked(addr: PhysAddr) -> Self {
        let mut table_addr =
            nmm::map_alloc(addr.into(), core::mem::size_of::<T>(), MapFlags::WRITABLE)
                .expect("Failed to map ACPI table");

        let table: &mut SdtHeader = unsafe { &mut *(table_addr.as_mut_ptr()) };

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = nmm::map_alloc(addr.into(), length, MapFlags::WRITABLE)
                .expect("Failed to map full ACPI table");
            unsafe { nmm::unmap(table_addr, length) };
            table_addr = new_phys_map;
        }

        // FIXME: Again, flagrantly undefined behavior. I don't know how I never noticed this, but here we are.
        // speaking of..
        // TODO: Owned -> Unique. Owned doesn't stress the uniqueness of the pointer, but really it should.
        let table: Owned<T> = unsafe { Owned::new(table_addr.as_mut_ptr()) };

        let sdt = unsafe { Owned::new(table_addr.as_mut_ptr()) };

        Self { table, sdt }
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
        let addr = Owned::as_ptr(&self.table);
        unsafe { nmm::unmap(VirtAddr::from_ptr(addr), self.sdt.length as usize) };
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
