//! SDT (System Description Table) support. Provides abstractions for safely accessing ACPI SDT headers.
use core::{mem, pin::Pin};

use acpi::{
    AcpiError, AcpiTable,
    sdt::{SdtHeader, Signature},
};
use cake::Owned;
use x86_64::{PhysAddr, structures::paging::PageTableFlags};

use crate::memory::paging::phys::phys_mem::{PhysicalMemoryMap, map_address, remap_address};

/// A header for an ACPI SDT (System Description Table).
#[derive(Debug)]
pub struct TableHeader<'a> {
    _map: PhysicalMemoryMap,
    sdt: Owned<SdtHeader>,
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> TableHeader<'a> {
    /// Creates a new `TableHeader` from a physical address.
    /// # Safety
    /// The caller must ensure that the physical address is valid and that the table is not already mapped.
    pub unsafe fn new(p_address: PhysAddr) -> Self {
        let map = unsafe {
            map_address(
                p_address,
                size_of::<SdtHeader>() as u64,
                PageTableFlags::PRESENT,
            )
        }
        .expect("Failed to map ACPI table header");

        let inner = unsafe { Owned::new(&mut *(map.ptr() as *mut acpi::sdt::SdtHeader)) };
        let len = inner.length;
        if len as usize > mem::size_of::<SdtHeader>() {
            let new_map = remap_address(
                &map,
                len as u64,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            )
            .expect("Failed to map full ACPI table header");

            let inner = unsafe { Owned::new(&mut *(new_map.ptr() as *mut acpi::sdt::SdtHeader)) };
            return unsafe { Self::from_raw_parts(new_map, inner) };
        }

        unsafe { Self::from_raw_parts(map, inner) }
    }

    /// Creates a new `TableHeader` from raw parts.
    ///
    /// # Safety
    /// The caller must ensure that the physical memory map and SDT header are valid.
    pub unsafe fn from_raw_parts(map: PhysicalMemoryMap, sdt: Owned<SdtHeader>) -> Self {
        Self {
            _map: map,
            sdt,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Returns a reference to the table header.
    pub fn header(&self) -> &SdtHeader {
        &self.sdt
    }

    /// Returns a pointer to the table data, immediately following the header.
    pub fn table_ptr(&self) -> *const u8 {
        unsafe { (&*self.sdt as *const SdtHeader).add(1).cast() }
    }

    /// Tries to convert the table to a reference of the given type.
    /// Returns `None` if the signatures do not match.
    pub fn try_as<T>(&self) -> Result<Pin<&T>, AcpiError>
    where
        T: AcpiTable,
    {
        unsafe { self.sdt.validate(T::SIGNATURE)? };
        let ptr = &*self.sdt as *const SdtHeader as *const T;
        Ok(unsafe { Pin::new_unchecked(&*ptr) })
    }

    /// Tries to convert the table to a mutable reference of the given type.
    /// Returns `None` if the signatures do not match.
    pub fn try_as_mut<T>(&mut self) -> Result<Pin<&mut T>, AcpiError>
    where
        T: AcpiTable,
    {
        unsafe { self.sdt.validate(T::SIGNATURE)? };
        let ptr = &mut *self.sdt as *mut SdtHeader as *mut T;
        Ok(unsafe { Pin::new_unchecked(&mut *ptr) })
    }

    /// Validates the table signature with the given `sig`.
    pub fn validate(&self, sig: Signature) -> Result<(), AcpiError> {
        unsafe { self.sdt.validate(sig) }
    }
}
