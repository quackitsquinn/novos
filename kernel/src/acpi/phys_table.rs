use x86_64::{
    structures::paging::{PageTable, PageTableFlags},
    PhysAddr,
};

use crate::memory::paging::phys::phys_mem::{map_address, MapError, PhysicalMemoryMap};

pub struct PhysicalTable<T>
where
    T: 'static,
{
    table: &'static T,
    mapping: PhysicalMemoryMap,
    size: u64,
}

impl<T> PhysicalTable<T>
where
    T: 'static,
{
    pub unsafe fn new(table: PhysAddr) -> Result<PhysicalTable<T>, MapError> {
        let size = core::mem::size_of::<T>() as u64;
        // Don't allow writes because table is &'static rather than &'static mut
        let mapping = map_address(table, size, PageTableFlags::PRESENT)?;
        Ok(PhysicalTable {
            table: unsafe { &*mapping.ptr().cast() },
            mapping,
            size,
        })
    }

    pub fn table(&self) -> &'static T {
        self.table
    }
    /// Remap the table to a new size. This is useful for ACPI tables that do not have a fixed size and need to be resized. (e.g. RSDT)
    pub unsafe fn remap(&mut self, new_size: u64) -> Result<(), MapError> {
        assert!(
            new_size > self.size,
            "New size must be greater than current size"
        );
        self.mapping = map_address(self.mapping.phys_addr(), new_size, PageTableFlags::PRESENT)?;
        self.size = new_size;
        Ok(())
    }
}
