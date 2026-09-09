//! Contains the core types and structures related to paging, such as page table entries, page tables, and the layout of the page table hierarchy. It also defines the virtual and physical address types used by the architecture.
pub mod accessor;
pub(crate) mod asm;
pub mod builder;
mod fragment;
pub mod index;
pub(crate) mod limine;
pub mod map;
pub mod operation;
pub mod primitives;
mod table;

pub use table::{PageTable, PageTableEntry};

use cake::log::{trace, warn};
pub use index::PageTableIndex;

use crate::{
    MapFlags, MapSource, MemError,
    arch::{Mapper, PageEntryType},
    map_with_operation,
    paging::{
        fragment::GreedyFragmentMapper,
        map::{
            DataAllocator, Flush, FullProvider, GlobalMemoryProvider, MemoryMapper, PhysLinear,
            SizedMemoryMapper, Unmapped,
        },
        operation::{Chain, OperationAllSizes, ZeroMemory},
        primitives::{AnyFragment, PageClass, PrimitiveClass},
    },
};

pub use primitives::Frame;
pub use primitives::MemoryRange;
pub use primitives::Page;
pub use primitives::{Address, AddressExt};
pub use primitives::{FragmentSize, Large, Medium, MemoryFragment, Small};
pub use primitives::{PhysAddr, VirtAddr};

/// The type used for page table entries in the current architecture.
pub type Table = [PageEntryType; crate::arch::ENTRY_COUNT];

/// A trait for managing ranges of memory primitives, such as pages. T
// I wish we didn't also have to specify the address space here..
pub unsafe trait FragmentManager<T: MemoryFragment<S>, S: FragmentSize> {
    /// Allocates a range of memory of the specified size and alignment, returning the starting address of the allocated range.
    fn allocate_fragment(&mut self) -> Result<T, MemError>;
    /// Deallocates a previously allocated range of memory, given the starting address and size of the range.
    fn deallocate_fragment(&mut self, primitive: T);
}

/// A trait for managing ranges of memory primitives of all sizes (small, medium, and large).
pub trait FullManager<C: PrimitiveClass>:
    FragmentManager<C::Fragment<Small>, Small>
    + FragmentManager<C::Fragment<Medium>, Medium>
    + FragmentManager<C::Fragment<Large>, Large>
{
    /// Allocates a small memory primitive (typically 4KB in size for x86_64 architecture).
    fn allocate_small(&mut self) -> Result<C::Fragment<Small>, MemError> {
        self.allocate_fragment()
    }

    /// Allocates a medium memory primitive (typically 2MB in size for x86_64 architecture).
    fn allocate_medium(&mut self) -> Result<C::Fragment<Medium>, MemError> {
        self.allocate_fragment()
    }

    /// Allocates a large memory primitive (typically 1GB in size for x86_64 architecture).
    fn allocate_large(&mut self) -> Result<C::Fragment<Large>, MemError> {
        self.allocate_fragment()
    }
}

/// Maps a memory primitive (such as a frame) to a page with the specified flags, using the provided frame allocator to allocate any necessary intermediate page tables.
#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub fn map_primitive<S, A>(
    src: Frame<S>,
    dst: Page<S>,
    flags: MapFlags,
    frame_allocator: &mut A,
) -> Result<Flush, MemError>
where
    S: FragmentSize,
    A: FragmentManager<Frame<Small>, Small>,
    Mapper: SizedMemoryMapper<S>,
{
    trace!(
        "Mapping frame {:?} to page {:?} with flags {}",
        src, dst, flags
    );

    let active_as = asm::active();
    let mut mapper = active_as.mapper().unwrap();

    mapper.map_primitive(dst, src, flags, frame_allocator)
}

/// Unmaps a page, returning the frame that was mapped to it before, or an error if the page was not mapped.
///
/// # Safety
///
/// The caller must ensure that there are no currently living references to the memory that was mapped to the page being unmapped,
/// as accessing that memory afterwards is undefined behavior.
#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub unsafe fn unmap_primitive<S>(dst: Page<S>) -> Result<Unmapped<S>, MemError>
where
    S: FragmentSize,
    Mapper: SizedMemoryMapper<S>,
{
    trace!("Unmapping page {:?}", dst);

    let active_as = asm::active();
    let mut mapper = active_as.mapper().unwrap();

    unsafe { mapper.unmap_primitive(dst) }
}

pub(crate) unsafe fn map_from<P>(
    base: VirtAddr,
    len: u64,
    flags: MapFlags,
    provider: &mut P,
) -> Result<(), MemError>
where
    P: FullProvider,
{
    trace!(
        "Mapping from base address {:x?} with length {:?} and flags {:?}",
        base.as_u64(),
        len,
        flags
    );

    let active_as = asm::active();
    let mut mapper = active_as.mapper().unwrap();

    unsafe { mapper.map_from(base, len, flags, provider) }
}

pub(crate) unsafe fn map_unchecked(
    dest: VirtAddr,
    src: MapSource,
    byte_size: usize,
    flags: MapFlags,
) -> Result<(), MemError> {
    match src {
        MapSource::Direct(phys_base) => {
            trace!(
                "Mapping physical memory at address {:#x} to virtual address {:#x} with size {} bytes and flags {:?}",
                phys_base.as_u64(),
                dest.as_u64(),
                byte_size,
                flags
            );
            unsafe {
                map_from(
                    dest,
                    byte_size as u64,
                    flags,
                    &mut PhysLinear(phys_base, &mut GlobalMemoryProvider),
                )?
            };
        }
        MapSource::Anon { zero } => unsafe {
            trace!(
                "Mapping anonymous memory at virtual address {:#x} with size {} bytes and flags {:?}",
                dest.as_u64(),
                byte_size,
                flags
            );
            if !zero {
                return map_from(
                    dest,
                    byte_size as u64,
                    flags | MapFlags::DEALLOCATE,
                    &mut DataAllocator(&mut GlobalMemoryProvider),
                );
            } else {
                return map_with_operation(dest, src, byte_size, flags, ZeroMemory);
            }
        },
    }

    Ok(())
}

pub(crate) unsafe fn unmap_unchecked(
    virt_base: VirtAddr,
    byte_size: usize,
) -> Result<(), MemError> {
    let mapper = GreedyFragmentMapper::<PageClass>::new(virt_base, byte_size as u64);

    fn unmap<S: FragmentSize>(page: Page<S>) -> Result<(), MemError>
    where
        Mapper: SizedMemoryMapper<S>,
    {
        let mut ent = unsafe { unmap_primitive(page)? };
        ent.flush();
        let mut pmm = asm::physical_memory_manager();
        if ent.flags.contains(MapFlags::DEALLOCATE) {
            pmm.deallocate_fragment(ent.frame);
        }

        for parent in ent.parent_tables.iter() {
            pmm.deallocate_fragment(*parent);
        }

        Ok(())
    }

    for frag in mapper {
        match frag {
            AnyFragment::Small(page_prim) => {
                unmap(page_prim)?;
            }
            AnyFragment::Medium(page_prim) => {
                unmap(page_prim)?;
            }
            AnyFragment::Large(page_prim) => {
                unmap(page_prim)?;
            }
        }
    }

    Ok(())
}

/// Maps a range of virtual addresses to physical frames, using the provided frame allocator for any necessary allocations of page tables,
/// and executes the given memory mapping operations for each mapping.
pub unsafe fn map_with_operation_unchecked(
    dest: VirtAddr,
    src: MapSource,
    byte_size: usize,
    flags: MapFlags,
    operation: impl OperationAllSizes,
) -> Result<(), MemError> {
    if matches!(src, MapSource::Anon { zero: false }) && !flags.contains(MapFlags::WRITABLE) {
        warn!(
            "nmm::map_with_operation: Mapping anonymous memory without zeroing and without the writable flag makes the mapping useless without undefined behavior."
        )
    }

    match src {
        MapSource::Direct(phys_base) => {
            trace!(
                "Mapping physical memory at address {:#x} to virtual address {:#x} with size {} bytes and flags {:?}",
                phys_base.as_u64(),
                dest.as_u64(),
                byte_size,
                flags
            );
            unsafe {
                map_from(
                    dest,
                    byte_size as u64,
                    flags,
                    &mut PhysLinear(phys_base, &mut *asm::physical_memory_manager()),
                )
            }
        }
        MapSource::Anon { zero } => unsafe {
            trace!(
                "Mapping anonymous memory at virtual address {:#x} with size {} bytes and flags {:?}",
                dest.as_u64(),
                byte_size,
                flags
            );

            if !zero {
                return map_from(
                    dest,
                    byte_size as u64,
                    flags | MapFlags::DEALLOCATE,
                    &mut DataAllocator(&mut *asm::physical_memory_manager()),
                );
            } else {
                let as_guard = asm::active();
                let mut mapper = as_guard.mapper().unwrap();
                return mapper.map_from_with_operation(
                    dest,
                    byte_size as u64,
                    flags,
                    &mut DataAllocator(&mut *asm::physical_memory_manager()),
                    Chain(ZeroMemory, operation),
                );
            }
        },
    }
}
