use core::mem;

use cake::log::error;

use crate::{
    MapFlags, MemError,
    arch::RecursivePageTable,
    paging::{
        AddressExt, FragmentManager, FragmentSize, Frame, Large, Page, PageTable, PageTableEntry,
        Small,
        accessor::{self, PagetableAccessor},
        asm::{self, InactiveAddressSpace},
        map::{Flush, MemoryMapper, SizedMemoryMapper},
        recursive_entry,
    },
};

/// Represents an address space who's page structure is currently mapped into one of the auxiliary recursive slots of the current address space.
///
/// This allows for direct access to the page tables of the mounted address space, enabling operations such as
/// mapping and unmapping pages within that address space.
pub struct MountedAddressSpace {
    pub(crate) l4_table_frame: Frame<Small>,
    pub(crate) scratch_page: Page<Large>,
    pub(crate) address_space: RecursivePageTable<'static>,
    pub(crate) rem: crate::paging::RecursiveEntryManager,
}

impl MountedAddressSpace {
    pub(crate) fn try_mount_inactive(
        mut ias: InactiveAddressSpace,
    ) -> Result<Self, (crate::paging::MemError, InactiveAddressSpace)> {
        match unsafe { mount(&mut ias) } {
            Ok(mas) => Ok(mas),
            Err(e) => Err((e, ias)),
        }
    }

    pub fn unmount(mut self) -> Result<InactiveAddressSpace, crate::paging::MemError> {
        unsafe { unmount_no_consume(&mut self) }
    }

    pub fn free(mut self) -> Result<(), crate::paging::MemError> {
        unsafe { cleanup_mapped_address_space(&mut self) }
    }
}

impl<S> SizedMemoryMapper<S> for MountedAddressSpace
where
    S: FragmentSize,
    RecursivePageTable<'static>: SizedMemoryMapper<S>,
{
    fn map_primitive<A>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: MapFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: FragmentManager<Frame<Small>, Small>,
    {
        self.address_space
            .map_primitive(page, frame, flags, allocator)
    }

    unsafe fn unmap_primitive(
        &mut self,
        page: Page<S>,
    ) -> Result<crate::paging::map::Unmapped<S>, MemError> {
        unsafe { self.address_space.unmap_primitive(page) }
    }
}

impl MemoryMapper for MountedAddressSpace {}

impl PagetableAccessor for MountedAddressSpace {
    fn read_l4_table(&self) -> Result<crate::paging::VirtAddr, MemError> {
        self.address_space.read_l4_table()
    }

    fn read_l3_table(
        &self,
        l4_index: crate::paging::PageTableIndex,
    ) -> Result<crate::paging::VirtAddr, MemError> {
        self.address_space.read_l3_table(l4_index)
    }

    fn read_l2_table(
        &self,
        l4_index: crate::paging::PageTableIndex,
        l3_index: crate::paging::PageTableIndex,
    ) -> Result<crate::paging::VirtAddr, MemError> {
        self.address_space.read_l2_table(l4_index, l3_index)
    }

    fn read_l1_table(
        &self,
        l4_index: crate::paging::PageTableIndex,
        l3_index: crate::paging::PageTableIndex,
        l2_index: crate::paging::PageTableIndex,
    ) -> Result<crate::paging::VirtAddr, MemError> {
        self.address_space
            .read_l1_table(l4_index, l3_index, l2_index)
    }
}

pub(super) unsafe fn mount(
    ias: &mut InactiveAddressSpace,
) -> Result<MountedAddressSpace, crate::paging::MemError> {
    let recursive_entry = match asm::reserve_recursive_slot() {
        Ok(entry) => entry,
        Err(e) => return Err(e),
    };
    let a_as = asm::active();
    let mut mapper_lock = a_as.mapper.lock_inner_mapper();
    let pml4 = mapper_lock.root_table_mut();
    unsafe {
        pml4.set_entry(
            recursive_entry,
            PageTableEntry::new(ias.l4_table_frame, MapFlags::WRITABLE),
        );
    }
    drop(mapper_lock);
    drop(a_as);
    let pml4_vaddr = accessor::build_vaddress(
        recursive_entry,
        recursive_entry,
        recursive_entry,
        recursive_entry,
    );
    let pml4 = unsafe { &mut *(pml4_vaddr.as_mut_ptr::<PageTable>()) };
    let recusive_table = unsafe { RecursivePageTable::new(pml4, recursive_entry) };

    Ok(MountedAddressSpace {
        l4_table_frame: ias.l4_table_frame,
        scratch_page: ias.scratch_page,
        address_space: recusive_table,
        rem: ias.rem.clone(),
    })
}

unsafe fn cleanup_mapped_address_space(
    mas: &mut MountedAddressSpace,
) -> Result<(), crate::paging::MemError> {
    let a_as = asm::active();
    let mut mapper_lock = a_as.mapper.lock_inner_mapper();
    let mut pmm = asm::pmm();
    unsafe {
        accessor::cleanup_l3(
            mas.address_space.recursive_index(),
            &mut *mapper_lock,
            &mut *pmm,
            false,
        )?;
    }

    // Unmount the address space and instantly forget it to avoid a double free.
    mem::forget(unsafe { unmount_no_consume(mas)? });
    Ok(())
}

unsafe fn unmount_no_consume(
    mas: &mut MountedAddressSpace,
) -> Result<InactiveAddressSpace, crate::paging::MemError> {
    let recursive_entry = mas.address_space.recursive_index();
    let a_as = asm::active();
    let mut mapper_lock = a_as.mapper.lock_inner_mapper();
    let pml4 = mapper_lock.root_table_mut();
    unsafe {
        pml4.set_entry(recursive_entry, PageTableEntry::empty());
    }
    drop(mapper_lock);
    drop(a_as);
    match unsafe { asm::release_recursive_slot(recursive_entry) } {
        Ok(_) => {}
        Err(e) => return Err(e),
    }
    Ok(InactiveAddressSpace {
        l4_table_frame: mas.l4_table_frame,
        scratch_page: mas.scratch_page,
        bootstrap_hhdm_offset: None,
        rem: mas.rem.clone(),
    })
}
impl Drop for MountedAddressSpace {
    fn drop(&mut self) {
        if let Err(e) = unsafe { cleanup_mapped_address_space(self) } {
            error!("Failed to cleanup mapped address space: {:?}", e);
        }
    }
}
