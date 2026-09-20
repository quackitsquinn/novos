//! Address Space Management (ASM) module for nmm.

use core::{
    cell::RefCell,
    mem::{self, transmute},
};

use cake::{OnceMutex, OnceMutexGuard, OnceRwLock, OnceRwReadGuard, log::info};

use crate::{
    MapFlags, MemError,
    arch::{self},
    bitmap::{PhysicalMemoryManager, VirtualMemoryManager},
    paging::{
        Address, AddressExt, FragmentSize, Frame, Large, MemoryFragment, Page, PageTable,
        PageTableEntry, PageTableIndex, RecursiveEntryManager, Small,
        accessor::{self},
        map::{LocalMemoryMapper, MapperMut, SizedMemoryMapper},
        recursive::RecursivePageTable,
    },
};

mod inactive;
mod mounted;

pub use inactive::InactiveAddressSpace;
pub use mounted::MountedAddressSpace;

static ADDRESS_SPACE: OnceRwLock<AddressSpace> = OnceRwLock::new();
static VIRTUAL_MEMORY_MANAGER: OnceMutex<VirtualMemoryManager<'static>> =
    OnceMutex::uninitialized();
static PHYSICAL_MEMORY_MANAGER: OnceMutex<PhysicalMemoryManager> = OnceMutex::uninitialized();

pub(crate) struct AddressSpace {
    pub(crate) mapper: LocalMemoryMapper<RecursivePageTable<'static>>,
    pub mut(crate) l4_table_frame: Frame<Small>,
    pub mut(crate) l4_table: Page<Small>,
    pub mut(crate) scratch_page: Page<Large>,
    rem: RefCell<RecursiveEntryManager>,
}

impl AddressSpace {
    pub(crate) fn new(
        mapper: RecursivePageTable<'static>,
        l4_table_frame: Frame<Small>,
        scratch_page: Page<Large>,
    ) -> Self {
        let l4_table = mapper.p4().as_page();
        Self {
            mapper: LocalMemoryMapper::new(mapper),
            l4_table_frame,
            scratch_page,
            l4_table,
            rem: RefCell::new(RecursiveEntryManager::default()),
        }
    }

    pub(crate) fn without_vmm(
        mapper: RecursivePageTable<'static>,
        l4_table_frame: Frame<Small>,
        scratch_page: Page<Large>,
    ) -> Self {
        let l4_table = mapper.p4().as_page();
        Self {
            mapper: LocalMemoryMapper::new(mapper),
            l4_table_frame,
            l4_table,
            scratch_page,
            rem: RefCell::new(RecursiveEntryManager::default()),
        }
    }

    pub(crate) fn mapper(&self) -> Option<MapperMut<'_, RecursivePageTable<'static>>> {
        if self.l4_table_frame == arch::pml4_phys() {
            Some(self.mapper.get_mapper())
        } else {
            None
        }
    }

    pub fn reserve_recursive_slot(&self) -> Result<PageTableIndex, MemError> {
        let mut rem = self.rem.borrow_mut();
        rem.reserve()
            .ok_or(MemError::Other("no recursive slots available"))
    }

    pub unsafe fn release_recursive_slot(&self, idx: PageTableIndex) -> Result<(), MemError> {
        let mut rem = self.rem.borrow_mut();
        unsafe { rem.release(idx) };
        Ok(())
    }
}

pub(crate) unsafe fn set_active(new_space: AddressSpace) {
    let mut new_as = Some(new_space);
    // Try to initialize, if we fail, just overwrite the existing address space.
    ADDRESS_SPACE.init(|| new_as.take().unwrap());

    if let Some(new_as) = new_as {
        // If we failed to initialize, we need to overwrite the existing address space.
        let mut old_as = ADDRESS_SPACE.write();
        *old_as = new_as;
    }
}

/// Sets the global physical memory manager. This function should only be called once during system initialization.
/// Returns true if the physical memory manager was successfully set, false if it was already set.
pub(crate) fn set_pmm(pmm: PhysicalMemoryManager) -> bool {
    let mut pmm = Some(pmm);
    PHYSICAL_MEMORY_MANAGER.call_init(|| pmm.take().unwrap());
    pmm.is_none()
}

pub(crate) fn active() -> OnceRwReadGuard<'static, AddressSpace> {
    ADDRESS_SPACE.read()
}

pub(crate) fn pmm() -> OnceMutexGuard<'static, PhysicalMemoryManager> {
    PHYSICAL_MEMORY_MANAGER.get()
}

pub(crate) fn set_vmm(vmm: VirtualMemoryManager<'static>) -> bool {
    let mut vmm = Some(vmm);
    VIRTUAL_MEMORY_MANAGER.call_init(|| vmm.take().unwrap());
    vmm.is_none()
}

pub(crate) fn vmm() -> Result<OnceMutexGuard<'static, VirtualMemoryManager<'static>>, MemError> {
    VIRTUAL_MEMORY_MANAGER
        .try_get()
        .ok_or(MemError::Uninit("virtual memory manager"))
}

pub(crate) fn reserve_recursive_slot() -> Result<PageTableIndex, MemError> {
    let aspace = active();
    aspace.reserve_recursive_slot()
}

pub(crate) unsafe fn release_recursive_slot(idx: PageTableIndex) -> Result<(), MemError> {
    let aspace = active();
    unsafe { aspace.release_recursive_slot(idx) }
}

pub(crate) fn activate_inactive_space(
    space: inactive::InactiveAddressSpace,
) -> Result<(), MemError> {
    let pml4_page = accessor::build_vaddress(
        space.recursive_index,
        space.recursive_index,
        space.recursive_index,
        space.recursive_index,
    );
    let pml4 = unsafe { &mut *(pml4_page.as_mut_ptr::<PageTable>()) };
    let mapper = unsafe { RecursivePageTable::new(pml4, space.recursive_index) };

    let new_space = AddressSpace::new(mapper, space.l4_table_frame, space.scratch_page);
    unsafe { set_active(new_space) };

    if space.is_bootstrap {
        mem::forget(space);
        return Ok(());
    }

    // Hold our breath..
    info!(target: "nmm", "Activating new address space with L4 table frame: {:#x}", space.l4_table_frame.start_address().as_u64());
    unsafe {
        arch::set_root_table(space.l4_table_frame);
    }
    // Thank god it didn't explode. Now we can breathe again.
    info!(target: "nmm", "New address space activated with L4 table frame: {:#x}", space.l4_table_frame.start_address().as_u64());

    Ok(())
}

/// A trait for types that can own a mapping of memory into an address space.
///
/// This trait is used to allow types to map their internal state into an address space, such as the memory manager's internal data structures.
pub trait MappingOwner {
    fn map_into(&mut self, mapper: &mut MountedAddressSpace) -> Result<(), MemError>;
}

/// Zeros out the given frame by mapping it to a known V.A. and writing zeros to it.
///
/// This requires access to the scratch page, so this cannot be called from within a [with_scratch_frame] call.
///
/// # Safety
///
/// The caller must ensure that the given frame obeys the following constraints:
/// - The frame must be in a valid memory region, e.g. not RESERVED or BAD_MEMORY.
/// - The frame must not be mapped to to memory that is currently in use, e.g. by the kernel or a module.
pub(crate) unsafe fn zero_frame<S>(frame: Frame<S>) -> Result<(), MemError>
where
    S: FragmentSize,
    RecursivePageTable<'static>: SizedMemoryMapper<S>,
{
    unsafe {
        map_with_scratch_page(frame, MapFlags::WRITABLE, |page| {
            let ptr = page.start_address().as_mut_ptr::<u8>();
            core::ptr::write_bytes(ptr, 0, S::SIZE as usize);
        })
    }
}

/// Executes the given closure with a scratch page mapped to the given frame.
/// The scratch page is a known virtual address that can be used to temporarily map physical memory.
///
/// **The given page WILL NOT be valid after the closure returns.**
pub(crate) unsafe fn map_with_scratch_page<S, F, R>(
    src: Frame<S>,
    flags: MapFlags,
    f: F,
) -> Result<R, MemError>
where
    S: FragmentSize,
    F: FnOnce(Page<S>) -> R,
    RecursivePageTable<'static>: SizedMemoryMapper<S>,
{
    let dst = {
        let active_as = active();
        let dst = unsafe { transmute::<Page<Large>, Page<S>>(active_as.scratch_page) };
        let mut mapper = active_as.mapper().ok_or(MemError::Uninit("mapper"))?;
        let mut pmm = pmm();

        mapper
            .map_primitive(dst, src, flags, None, &mut *pmm)?
            .flush();
        dst
    };

    let res = f(dst);

    let active_as = active();
    let mut mapper = active_as.mapper().ok_or(MemError::Uninit("mapper"))?;
    unsafe {
        mapper
            .unmap_primitive(dst)
            .expect("failed to unmap just mapped region")
            .flush()
    };

    Ok(res)
}

pub(crate) unsafe fn init_table(
    l4: Frame<Small>,
    recursive_idx: PageTableIndex,
    zero: bool,
) -> Result<(), MemError> {
    unsafe {
        map_with_scratch_page(l4, MapFlags::WRITABLE, |page| {
            let pml4 = &mut *page.start_address().as_mut_ptr::<PageTable>();

            if zero {
                pml4.zero();
            }

            pml4.set_entry(recursive_idx, PageTableEntry::new(l4, MapFlags::WRITABLE));
        })
    }
}
