//! Architecture-specific types and implementations for x86_64.

pub mod addr;
pub(crate) mod api;
mod conv;
mod mapper;
mod offset;
mod recursive;

pub use mapper::Mapper;

use core::arch::asm;

use bitflags::bitflags;

pub use addr::{PhysAddr, VirtAddr};
use cake::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// This submodule exists purely to reduce name collisions as the internal implementation of x86_64
// is very similar to nmm (as i roughly modeled nmm after x86_64 and redox's memory management),
// so there are a lot of similar types and functions, and it would be very easy for them to collide if they were all in the same module.
//
// It's a bit verbose, but it's easier than dealing with the name collisions and weird compiler errors that would arise.
mod arch_lib {}

use crate::{
    MapFlags, MemError,
    arch::x86_64::conv::XFrameAllocator,
    paging::{
        Frame, Page, PrimitiveRangeManager, PrimitiveSize, Small,
        map::{Flush, MemoryMapper},
    },
};

/// The width of virtual addresses in bits for x86_64 architecture.
pub const VIRTUAL_ADDRESS_WIDTH: u8 = 48;
/// The maximum valid virtual address for x86_64 architecture.
pub const VIRTUAL_ADDRESS_MAX: u64 = (1 << VIRTUAL_ADDRESS_WIDTH) - 1;
/// The width of physical addresses in bits for x86_64 architecture.
pub const PHYSICAL_ADDRESS_WIDTH: u8 = 52;
/// The maximum valid physical address for x86_64 architecture.
pub const PHYSICAL_ADDRESS_MAX: u64 = (1 << PHYSICAL_ADDRESS_WIDTH) - 1;
/// The number of bits used for indexing into page tables at each level.
pub const TABLE_INDEX_BITS: usize = 9;
/// The number of entries in a page table for x86_64 architecture.
pub const ENTRY_COUNT: usize = L1_PAGE_SIZE as usize / core::mem::size_of::<u64>();
/// A page table entry for x86_64 architecture, represented as a 64-bit value.
pub type PageEntryType = u64;
/// The size of a level 1 page (4KB) for x86_64 architecture.
pub const L1_PAGE_SIZE: u64 = 4096;
/// The size of a level 2 page (2MB) for x86_64 architecture.
pub const L2_PAGE_SIZE: u64 = L1_PAGE_SIZE * ENTRY_COUNT as u64;
/// The size of a level 3 page (1GB) for x86_64 architecture.
pub const L3_PAGE_SIZE: u64 = L2_PAGE_SIZE * ENTRY_COUNT as u64;

pub type ArchEntryFlags = PageTableFlags;

bitflags! {
    /// Page table entry flags for x86_64 architecture.
    #[repr(transparent)]
    pub struct PageTableFlags: u64 {
        /// The page is present in memory.
        const PRESENT         = 1 << 0;
        /// The page is writable.
        const WRITABLE        = 1 << 1;
        /// The page is accessible from userspace.
        const USER_ACCESSIBLE = 1 << 2;
        /// Write-through caching enabled.
        const WRITE_THROUGH   = 1 << 3;
        /// Cache disabled for this page.
        const NO_CACHE       = 1 << 4;
        /// The page has been accessed.
        const ACCESSED        = 1 << 5;
        /// The page has been written to.
        const DIRTY           = 1 << 6;
        /// This is a huge page (2MB or 1GB).
        const HUGE_PAGE       = 1 << 7;
        /// The page is global and not flushed from TLB on CR3 reload.
        const GLOBAL          = 1 << 8;
        /// No-execute flag; if set, code execution is not allowed from this page.
        const NO_EXECUTE      = 1 << 63;
    }
}

/// An error that originate from architecture-specific operations in the memory manager. This is the error type for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ArchError {
    #[error("Parent page table entry is a huge page, cannot map to it")]
    /// An error indicating that a mapping operation failed because the parent page table entry is a huge page, which cannot be used for mapping.
    ParentEntryHugePage,
}

static ACTIVE_PAGETABLE: RwLock<Option<Mapper>> = RwLock::new(None);

pub(crate) fn mapper_read() -> RwLockReadGuard<'static, Option<Mapper>> {
    ACTIVE_PAGETABLE.read()
}

pub(crate) fn mapper_mut() -> RwLockWriteGuard<'static, Option<Mapper>> {
    ACTIVE_PAGETABLE.write()
}

pub(crate) unsafe fn set_mapper(mapper: Mapper) {
    *ACTIVE_PAGETABLE.write() = Some(mapper);
}

pub(crate) fn map_primitive<S, A>(
    src: Frame<S>,
    dst: Page<S>,
    flags: MapFlags,
    frame_allocator: &mut A,
) -> Result<Flush, MemError>
where
    S: PrimitiveSize,
    A: PrimitiveRangeManager<Frame<Small>, Small>,
    Mapper: MemoryMapper<S>,
{
    let mut mapper_guard = mapper_mut();
    let mapper = mapper_guard
        .as_mut()
        .ok_or(MemError::Uninit("global memory mapper"))?;

    mapper.map(dst, src, flags, frame_allocator)
}

pub(crate) unsafe fn unmap_primitive<S>(dst: Page<S>) -> Result<(Frame<S>, Flush), MemError>
where
    S: PrimitiveSize,
    Mapper: MemoryMapper<S>,
{
    let mut mapper_guard = mapper_mut();
    let mapper = mapper_guard
        .as_mut()
        .ok_or(MemError::Uninit("global memory mapper"))?;

    unsafe { mapper.unmap(dst) }
}

pub(crate) unsafe fn do_flush(addr: VirtAddr) {
    unsafe { asm!("invlpg [{}]", in(reg) addr.as_u64(), options(nostack, preserves_flags)) };
}

#[cfg(test)]
mod tests {

    #[test]
    fn assert_flags_correctness() {
        use super::PageTableFlags as OurFlags;
        use x86_64::structures::paging::page_table::PageTableFlags as X86Flags;
        macro_rules! check {
            ($name: ident) => {
                assert_eq!(OurFlags::$name.bits(), X86Flags::$name.bits(), "Flag {} does not match!", stringify!($name));
            };
            (($($name: ident),+)) => {
                $(
                    check!($name);
                )+
            }
        }

        check!((
            PRESENT,
            WRITABLE,
            USER_ACCESSIBLE,
            WRITE_THROUGH,
            NO_CACHE,
            ACCESSED,
            DIRTY,
            HUGE_PAGE,
            GLOBAL,
            NO_EXECUTE
        ));
    }
}
