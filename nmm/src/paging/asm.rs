//! Address Space Management (ASM) module for nmm.

use cake::{
    MappedMutexGuard, Mutex, MutexGuard, OnceMutex, OnceMutexGuard, OnceRwLock, OnceRwReadGuard,
};

use crate::{
    arch,
    bitmap::{PhysicalMemoryManager, VirtualMemoryManager},
    paging::{Frame, PhysAddr, Small},
};

static ADDRESS_SPACE: OnceRwLock<AddressSpace> = OnceRwLock::new();
static PHYSICAL_MEMORY_MANAGER: OnceMutex<PhysicalMemoryManager> = OnceMutex::uninitialized();

pub(crate) struct AddressSpace {
    mapper: Mutex<arch::Mapper>,
    l4_table: Frame<Small>,
    vmm: Mutex<Option<VirtualMemoryManager<'static>>>,
}

impl AddressSpace {
    pub(crate) fn new(
        mapper: arch::Mapper,
        l4_table: Frame<Small>,
        vmm: Option<VirtualMemoryManager<'static>>,
    ) -> Self {
        Self {
            mapper: Mutex::new(mapper),
            l4_table,
            vmm: Mutex::new(vmm),
        }
    }

    pub(crate) fn without_vmm(mapper: arch::Mapper, l4_table: Frame<Small>) -> Self {
        Self {
            mapper: Mutex::new(mapper),
            l4_table,
            vmm: Mutex::new(None),
        }
    }

    pub(crate) fn mapper(&self) -> Option<MutexGuard<'_, arch::Mapper>> {
        // TODO: Change return to Option<MutexGuard> and comparing the pml4 register to self.l4_table to determine if the mapper is valid for the current address space.
        if self.l4_table == arch::pml4_phys() {
            Some(self.mapper.lock())
        } else {
            None
        }
    }

    pub(crate) fn l4_table(&self) -> &Frame<Small> {
        &self.l4_table
    }

    pub(crate) fn vmm(&self) -> Option<MappedMutexGuard<'_, VirtualMemoryManager<'static>>> {
        MutexGuard::try_map(self.vmm.lock(), |vmm| vmm.as_mut()).ok()
    }

    /// Sets the virtual memory manager for this address space. This function should only be called once during system initialization.
    pub(crate) fn set_vmm(&self, vmm: VirtualMemoryManager<'static>) {
        let mut vmm_guard = self.vmm.lock();
        *vmm_guard = Some(vmm);
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
pub(crate) fn set_physical_memory_manager(pmm: PhysicalMemoryManager) -> bool {
    let mut pmm = Some(pmm);
    PHYSICAL_MEMORY_MANAGER.call_init(|| pmm.take().unwrap());
    pmm.is_none()
}

pub(crate) fn active() -> OnceRwReadGuard<'static, AddressSpace> {
    ADDRESS_SPACE.read()
}

pub(crate) fn physical_memory_manager() -> OnceMutexGuard<'static, PhysicalMemoryManager> {
    PHYSICAL_MEMORY_MANAGER.get()
}
