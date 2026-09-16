use core::sync::atomic::AtomicBool;

use crate::{
    MapFlags, MemError,
    arch::{self, Mapper, RecursivePageTable},
    map_with_operation,
    paging::{
        Address, AddressExt, FragmentManager, FragmentSize, Frame, MemoryFragment, Page, PageTable,
        PageTableEntry, PageTableIndex, PhysAddr, Small,
        accessor::{cleanup_l3, cleanup_l4_range},
        asm::{self, AddressSpace},
        map::{DataAllocator, Flush, MemoryMapper, PhysLinear, SizedMemoryMapper, Unmapped},
        operation::ZeroMemory,
    },
};

pub struct RecursiveAddressSpaceBuilder<'a> {
    table: RecursivePageTable<'a>,
}

// TODO: needs to be core local when mp supported, since each core will have it's own pair of recursive entries.
static RECURSIVE_LOCKED: AtomicBool = AtomicBool::new(false);

impl<'a> RecursiveAddressSpaceBuilder<'a> {
    pub fn acquire_instance() -> Result<Self, MemError> {
        if RECURSIVE_LOCKED.swap(true, core::sync::atomic::Ordering::Acquire) {
            return Err(MemError::ResourceUnavailable(
                "RecursiveAddressSpaceBuilder",
            ));
        }

        let builder_pml4: Frame<Small> = crate::reserve_frame()?;
        unsafe { asm::zero_frame(builder_pml4)? };
        let active_as = asm::active();
        let mut mapper = active_as.mapper.lock_inner_mapper();
        let current_pml4 = mapper.root_table_mut();
        if current_pml4.read_entry(arch::RECURSIVE_SLOT1).is_present() {
            return Err(MemError::ResourceUnavailable(
                "RecursiveAddressSpaceBuilder: RECURSIVE_SLOT1 is already in use",
            ));
        }
        unsafe {
            current_pml4.set_entry(
                arch::RECURSIVE_SLOT1,
                PageTableEntry::new(builder_pml4, MapFlags::WRITABLE),
            )
        };

        Ok(Self {
            table: unsafe {
                RecursivePageTable::new(
                    &mut *active_as.l4_table.start_address().as_mut_ptr::<PageTable>(),
                    crate::arch::RECURSIVE_SLOT1,
                )
            },
        })
    }
}

impl<'a, S: FragmentSize> SizedMemoryMapper<S> for RecursiveAddressSpaceBuilder<'a>
where
    RecursivePageTable<'a>: SizedMemoryMapper<S>,
{
    fn map_primitive<A>(
        &mut self,
        dst: Page<S>,
        src: Frame<S>,
        flags: MapFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: FragmentManager<Frame<Small>, Small>,
    {
        self.table.map_primitive(dst, src, flags, allocator)
    }

    unsafe fn unmap_primitive(&mut self, page: Page<S>) -> Result<Unmapped<S>, MemError> {
        unsafe { self.table.unmap_primitive(page) }
    }
}

impl<'a> Drop for RecursiveAddressSpaceBuilder<'a> {
    fn drop(&mut self) {
        let active_as = asm::active();
        let mut pmm = asm::pmm();
        let mut mapper = active_as.mapper.lock_inner_mapper();
        unsafe {
            cleanup_l3(arch::RECURSIVE_SLOT1, &mut *mapper, &mut *pmm, false)
                .expect("failed to cleanup recursive address space");
        }
        let current_pml4 = mapper.root_table_mut();
        unsafe { current_pml4.set_entry(arch::RECURSIVE_SLOT1, PageTableEntry::empty()) };
    }
}
