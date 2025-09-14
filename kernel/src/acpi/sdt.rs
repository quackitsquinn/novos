use core::{mem, pin::Pin};

use acpi::{
    sdt::{SdtHeader, Signature},
    AcpiError, AcpiTable,
};
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

use crate::{
    memory::paging::{
        map,
        phys::phys_mem::{map_address, remap_address, unmap_address, PhysicalMemoryMap},
    },
    util::Owned,
};

#[derive(Debug)]
pub struct TableHeader<'a> {
    map: PhysicalMemoryMap,
    sdt: Owned<SdtHeader>,
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> TableHeader<'a> {
    pub unsafe fn new(p_address: PhysAddr) -> Self {
        let map = map_address(
            p_address,
            size_of::<SdtHeader>() as u64,
            PageTableFlags::PRESENT,
        )
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

    pub unsafe fn from_raw_parts(map: PhysicalMemoryMap, sdt: Owned<SdtHeader>) -> Self {
        Self {
            map,
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

    pub fn validate(&self, sig: Signature) -> Result<(), AcpiError> {
        unsafe { self.sdt.validate(sig) }
    }
}
