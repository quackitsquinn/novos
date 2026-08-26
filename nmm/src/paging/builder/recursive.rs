use core::sync::atomic::AtomicBool;

use crate::{
    MapFlags, MemError,
    arch::RecursivePageTable,
    paging::{
        Address, AddressExt, MemoryFragment, PageTable,
        PhysAddr, asm,
        builder::AddressSpaceBuilder,
        map::{DataAllocator, MemoryMapper, PhysLinear},
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
            panic!("RecursiveAddressSpaceBuilder instance already acquired");
        }

        let active_as = asm::active();
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

impl<'a> AddressSpaceBuilder for RecursiveAddressSpaceBuilder<'a> {
    fn map(
        &mut self,
        base: crate::paging::VirtAddr,
        source: super::Source,
        size: u64,
        map_flags: crate::MapFlags,
    ) -> Result<(), crate::MemError> {
        use super::Source as S;
        let mut pmm = asm::physical_memory_manager();
        match source {
            S::Allocate { should_zero } => unsafe {
                if should_zero {
                    self.table.map_from_with_operation(
                        base,
                        size,
                        map_flags | MapFlags::DEALLOCATE,
                        &mut DataAllocator(&mut *pmm),
                        ZeroMemory,
                    )
                } else {
                    self.table.map_from(
                        base,
                        size,
                        map_flags | MapFlags::DEALLOCATE,
                        &mut DataAllocator(&mut *pmm),
                    )
                }
            },
            S::Identity => {
                let phys_base = PhysAddr::try_new(base.as_u64()).ok_or(MemError::Other(
                    "failed to convert virt to phys for identity mapping",
                ))?;
                unsafe {
                    self.table.map_from(
                        base,
                        size,
                        map_flags,
                        &mut PhysLinear(phys_base, &mut *pmm),
                    )
                }
            }
            S::PhysAddr(phys_addr) => unsafe {
                self.table
                    .map_from(base, size, map_flags, &mut PhysLinear(phys_addr, &mut *pmm))
            },
        }
    }
}

impl<'a> Drop for RecursiveAddressSpaceBuilder<'a> {
    fn drop(&mut self) {
        RECURSIVE_LOCKED.store(false, core::sync::atomic::Ordering::Release);
    }
}
