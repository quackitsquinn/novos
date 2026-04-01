//! SDT (System Description Table) support. Provides abstractions for safely accessing ACPI SDT headers.

use acpi::{
    AcpiError, AcpiTable,
    sdt::{SdtHeader, Signature},
};
use cake::Owned;
use nmm::MapFlags;
use x86_64::{PhysAddr, structures::paging::PageTableFlags};

use crate::acpi::MappedTable;

/// A header for an ACPI SDT (System Description Table).
#[derive(Debug)]
pub struct TableHeader<'a> {
    sdt: Owned<SdtHeader>,
    physical_address: PhysAddr,
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> TableHeader<'a> {
    /// Creates a new `TableHeader` from a physical address.
    /// # Safety
    /// The caller must ensure that the physical address is valid and that the table is not already mapped.
    pub unsafe fn new(p_address: PhysAddr) -> Self {
        let map = nmm::map_alloc(
            p_address.into(),
            size_of::<SdtHeader>() as u64,
            MapFlags::empty(),
        )
        .expect("Failed to map ACPI table header");

        let inner = unsafe { Owned::new(&mut *(map.as_mut_ptr())) };

        unsafe { Self::from_raw(inner, p_address) }
    }

    /// Creates a new `TableHeader` from raw parts.
    ///
    /// # Safety
    /// The caller must ensure that the physical memory map and SDT header are valid.
    pub unsafe fn from_raw(sdt: Owned<SdtHeader>, p_addr: PhysAddr) -> Self {
        Self {
            sdt,
            physical_address: p_addr,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Tries to convert the table to a mutable reference of the given type.
    /// Returns `None` if the signatures do not match.
    pub fn to_table<T>(self) -> Result<MappedTable<T>, AcpiError>
    where
        T: AcpiTable,
    {
        // First validate the table signature
        unsafe { self.sdt.validate(T::SIGNATURE)? };

        let p_addr = self.physical_address;
        drop(self);

        unsafe { Ok(MappedTable::new_unchecked(p_addr)) }
    }

    /// Returns a reference to the table header.
    pub fn header(&self) -> &SdtHeader {
        &self.sdt
    }

    /// Returns a pointer to the table data, immediately following the header.
    pub fn table_ptr(&self) -> *const u8 {
        unsafe { (&*self.sdt as *const SdtHeader).add(1).cast() }
    }

    /// Validates the table signature with the given `sig`.
    pub fn validate(&self, sig: Signature) -> Result<(), AcpiError> {
        unsafe { self.sdt.validate(sig) }
    }
}
