use core::fmt;

use acpi::{sdt::SdtHeader, AcpiError, AcpiTable};
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

use crate::{
    memory::paging::phys::phys_mem::{map_address, unmap_address, PhysicalMemoryMap},
    util::Owned,
};

pub struct MappedTable<'a, T: AcpiTable> {
    table: Owned<T>,
    sdt: &'a SdtHeader,
    phys_range: PhysicalMemoryMap,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: AcpiTable> MappedTable<'a, T> {
    pub fn new(addr: PhysAddr) -> Result<Self, AcpiError> {
        let mut phys_map = map_address(
            addr,
            core::mem::size_of::<T>() as u64,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
        .expect("Failed to map ACPI table");

        let table: &mut SdtHeader = unsafe { &mut *(phys_map.ptr() as *mut SdtHeader) };

        if table.signature != T::SIGNATURE {
            return Err(AcpiError::SdtInvalidSignature(table.signature));
        }

        let length = table.length as usize;
        if length > core::mem::size_of::<T>() {
            let new_phys_map = map_address(
                addr,
                length as u64,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            )
            .expect("Failed to map full ACPI table");
            unmap_address(phys_map);
            phys_map = new_phys_map;
        }

        let table: Owned<T> = unsafe { Owned::new(phys_map.ptr().cast_mut().cast()) };

        table.validate()?;

        let sdt: &'a SdtHeader = unsafe { &*(phys_map.ptr() as *const SdtHeader) };

        Ok(Self {
            table,
            phys_range: phys_map,
            sdt,
            _marker: core::marker::PhantomData,
        })
    }

    pub fn table(&self) -> &T {
        &self.table
    }
}

impl<'a, T: AcpiTable> Drop for MappedTable<'a, T> {
    fn drop(&mut self) {
        unmap_address(self.phys_range);
    }
}

impl<'a, T: AcpiTable> fmt::Debug for MappedTable<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MappedTable")
            .field("sdt", &self.sdt)
            .field("phys_range", &self.phys_range)
            .finish()
    }
}
