//! Address Space Management (ASM) module for nmm.

use core::mem::transmute;

use cake::{
    MappedMutexGuard, Mutex, MutexGuard, OnceMutex, OnceMutexGuard, OnceRwLock, OnceRwReadGuard,
};

use crate::{
    MapFlags, MemError,
    arch::{self},
    bitmap::{PhysicalMemoryManager, VirtualMemoryManager},
    paging::{
        AddressExt, FragmentSize, Frame, Large, MemoryFragment, Page, Small,
        map::{LocalMemoryMapper, MapperMut, MemoryMapper, SizedMemoryMapper},
    },
};

static ADDRESS_SPACE: OnceRwLock<AddressSpace> = OnceRwLock::new();
static VIRTUAL_MEMORY_MANAGER: OnceMutex<VirtualMemoryManager<'static>> =
    OnceMutex::uninitialized();
static PHYSICAL_MEMORY_MANAGER: OnceMutex<PhysicalMemoryManager> = OnceMutex::uninitialized();

pub(crate) struct AddressSpace {
    pub(crate) mapper: LocalMemoryMapper<arch::Mapper>,
    pub mut(crate) l4_table_frame: Frame<Small>,
    pub mut(crate) l4_table: Page<Small>,
    pub mut(crate) scratch_page: Page<Large>,
}

impl AddressSpace {
    pub(crate) fn new(
        mapper: arch::Mapper,
        l4_table_frame: Frame<Small>,
        scratch_page: Page<Large>,
    ) -> Self {
        let l4_table = mapper.root_table().as_page();
        Self {
            mapper: LocalMemoryMapper::new(mapper),
            l4_table_frame,
            scratch_page,
            l4_table,
        }
    }

    pub(crate) fn without_vmm(
        mapper: arch::Mapper,
        l4_table_frame: Frame<Small>,
        scratch_page: Page<Large>,
    ) -> Self {
        let l4_table = mapper.root_table().as_page();
        Self {
            mapper: LocalMemoryMapper::new(mapper),
            l4_table_frame,
            l4_table,
            scratch_page,
        }
    }

    pub(crate) fn mapper(&self) -> Option<MapperMut<'_, arch::Mapper>> {
        if self.l4_table_frame == arch::pml4_phys() {
            Some(self.mapper.get_mapper())
        } else {
            None
        }
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
    arch::Mapper: SizedMemoryMapper<S>,
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
    arch::Mapper: SizedMemoryMapper<S>,
{
    let dst = {
        let active_as = active();
        let dst = unsafe { transmute::<Page<Large>, Page<S>>(active_as.scratch_page) };
        let mut mapper = active_as.mapper().ok_or(MemError::Uninit("mapper"))?;
        let mut pmm = pmm();

        mapper.map_primitive(dst, src, flags, &mut *pmm)?.flush();
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
