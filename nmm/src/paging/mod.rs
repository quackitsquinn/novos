//! Contains the core types and structures related to paging, such as page table entries, page tables, and the layout of the page table hierarchy. It also defines the virtual and physical address types used by the architecture.
pub(crate) mod asm;
pub mod builder;
mod fragment;
pub mod index;
pub(crate) mod limine;
pub mod map;
pub mod primitives;
mod table;

use bitflags::bitflags;
pub use table::{PageTable, PageTableEntry};

use cake::log::trace;
pub use index::PageTableIndex;

use crate::{
    MapFlags, MapSource, MemError,
    arch::{self, Mapper, PageEntryType},
    paging::{
        fragment::{GreedyFragmentMapper, JointFragmentMapper},
        map::{Flush, MemoryMapper, Unmapped},
        primitives::{AnyFragment, FrameClass, PageClass, PrimitiveClass},
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
#[allow(private_bounds)] // intentionally seal this
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

impl<C, T> FullManager<C> for T
where
    C: PrimitiveClass,
    T: FragmentManager<C::Fragment<Small>, Small>
        + FragmentManager<C::Fragment<Medium>, Medium>
        + FragmentManager<C::Fragment<Large>, Large>,
{
}

/// Maps a memory primitive (such as a frame) to a page with the specified flags, using the provided frame allocator to allocate any necessary intermediate page tables.
//#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub(crate) fn map_primitive<S, A>(
    src: Frame<S>,
    dst: Page<S>,
    flags: MapFlags,
    mapping_flags: EntryMappingFlags,
    frame_allocator: &mut A,
) -> Result<Flush, MemError>
where
    S: FragmentSize,
    A: FragmentManager<Frame<Small>, Small>,
    Mapper: MemoryMapper<S>,
{
    trace!(
        "Mapping frame {:?} to page {:?} with flags {:?}",
        src, dst, flags
    );

    let active_as = asm::active();
    let mut mapper = active_as.mapper().unwrap();

    mapper.map(dst, src, flags, mapping_flags, frame_allocator)
}

/// Unmaps a page, returning the frame that was mapped to it before, or an error if the page was not mapped.
///
/// # Safety
///
/// The caller must ensure that there are no currently living references to the memory that was mapped to the page being unmapped,
/// as accessing that memory afterwards is undefined behavior.
#[must_use = "The returned `Flush` should be flushed after the mapping operation to ensure that there are no stale mappings."]
pub(crate) unsafe fn unmap_primitive<S>(dst: Page<S>) -> Result<Unmapped<S>, MemError>
where
    S: FragmentSize,
    Mapper: MemoryMapper<S>,
{
    trace!("Unmapping page {:?}", dst);

    let active_as = asm::active();
    let mut mapper = active_as.mapper().unwrap();

    unsafe { mapper.unmap(dst) }
}

pub(crate) unsafe fn map_from<D>(
    base: VirtAddr,
    len: u64,
    flags: MapFlags,
    mapping_flags: EntryMappingFlags,
    data_allocator: &mut D,
) -> Result<(), MemError>
where
    D: FullManager<FrameClass>,
{
    trace!(
        "Mapping from base address {:x?} with length {:?} and flags {:?}",
        base.as_u64(),
        len,
        flags
    );

    let mapper = GreedyFragmentMapper::<PageClass>::new(base, len);
    for frag in mapper {
        match frag {
            AnyFragment::Small(prim) => {
                let frame = data_allocator.allocate_small()?;
                map_primitive(frame, prim, flags, mapping_flags, data_allocator)?.flush();
            }
            AnyFragment::Medium(prim) => {
                let frame = data_allocator.allocate_medium()?;
                map_primitive(frame, prim, flags, mapping_flags, data_allocator)?.flush();
            }
            AnyFragment::Large(prim) => {
                let frame = data_allocator.allocate_large()?;
                map_primitive(frame, prim, flags, mapping_flags, data_allocator)?.flush();
            }
        }
    }

    Ok(())
}

pub(crate) unsafe fn map_from_with_allocator<D, F>(
    base: VirtAddr,
    len: u64,
    flags: MapFlags,
    mapping_flags: EntryMappingFlags,
    data_allocator: &mut D,
    frame_allocator: &mut F,
) -> Result<(), MemError>
where
    D: FullManager<FrameClass>,
    F: FragmentManager<Frame<Small>, Small>,
{
    trace!(
        "Mapping from base address {:?} with length {:?} and flags {:?}",
        base, len, flags
    );

    let mapper = GreedyFragmentMapper::<PageClass>::new(base, len);
    for frag in mapper {
        match frag {
            AnyFragment::Small(prim) => {
                let frame = data_allocator.allocate_small()?;
                map_primitive(frame, prim, flags, mapping_flags, frame_allocator)?.flush();
            }
            AnyFragment::Medium(prim) => {
                let frame = data_allocator.allocate_medium()?;
                map_primitive(frame, prim, flags, mapping_flags, frame_allocator)?.flush();
            }
            AnyFragment::Large(prim) => {
                let frame = data_allocator.allocate_large()?;
                map_primitive(frame, prim, flags, mapping_flags, frame_allocator)?.flush();
            }
        }
    }

    Ok(())
}

bitflags! {
    /// The flags used for handling special cases in page table entries, such as anonymous mappings.
    /// This is a bitflag where the positions of the bits are not formally defined, and above a couple entries even guaranteed to exist.
    /// This may need to be refactored if we need more than a few flags (i believe the lower limit accounting for x86_64, aarch64, and riscv is 3 bits but it may be more idk)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EntryMappingFlags: u64 {
        const MAP_ANON = arch::PTE_FREE_BIT0;
    }
}

impl Default for EntryMappingFlags {
    fn default() -> Self {
        Self::empty()
    }
}

pub(crate) unsafe fn map_raw<F>(
    virt_base: VirtAddr,
    phys_base: PhysAddr,
    byte_size: usize,
    flags: MapFlags,
    mapping_flags: EntryMappingFlags,
    frame_alloc: &mut F,
) -> Result<(), MemError>
where
    F: FragmentManager<Frame<Small>, Small>,
{
    let mapper = JointFragmentMapper::new(virt_base, phys_base, byte_size as u64);

    for pair in mapper {
        match pair {
            (AnyFragment::Small(page_prim), AnyFragment::Small(phys_prim)) => {
                map_primitive(phys_prim, page_prim, flags, mapping_flags, frame_alloc)?.flush();
            }
            (AnyFragment::Medium(page_prim), AnyFragment::Medium(phys_prim)) => {
                map_primitive(phys_prim, page_prim, flags, mapping_flags, frame_alloc)?.flush();
            }
            (AnyFragment::Large(page_prim), AnyFragment::Large(phys_prim)) => {
                map_primitive(phys_prim, page_prim, flags, mapping_flags, frame_alloc)?.flush();
            }
            _ => unreachable!("non-matched fragments produced by mapper"),
        }
    }

    Ok(())
}

pub(crate) unsafe fn map_unchecked(
    dest: VirtAddr,
    src: MapSource,
    byte_size: usize,
    flags: MapFlags,
) -> Result<(), MemError> {
    let mut pmm_guard = asm::physical_memory_manager();
    let pmm = &mut *pmm_guard;

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
                map_raw(
                    dest,
                    phys_base,
                    byte_size,
                    flags,
                    EntryMappingFlags::empty(),
                    pmm,
                )?
            };
        }
        MapSource::Anon => unsafe {
            trace!(
                "Mapping anonymous memory at virtual address {:#x} with size {} bytes and flags {:?}",
                dest.as_u64(),
                byte_size,
                flags
            );
            map_from(
                dest,
                byte_size as u64,
                flags,
                EntryMappingFlags::MAP_ANON,
                pmm,
            )?;
        },
    }

    Ok(())
}

pub(crate) unsafe fn unmap_unchecked(
    virt_base: VirtAddr,
    byte_size: usize,
) -> Result<(), MemError> {
    let mapper = GreedyFragmentMapper::<PageClass>::new(virt_base, byte_size as u64);

    for frag in mapper {
        match frag {
            AnyFragment::Small(page_prim) => {
                let mut ent = unsafe { unmap_primitive(page_prim)? };
                ent.flush();
                if ent.mapping_flags.contains(EntryMappingFlags::MAP_ANON) {
                    let mut pmm = asm::physical_memory_manager();
                    pmm.deallocate_fragment(ent.frame);
                }
            }
            AnyFragment::Medium(page_prim) => {
                let mut ent = unsafe { unmap_primitive(page_prim)? };
                ent.flush();
                if ent.mapping_flags.contains(EntryMappingFlags::MAP_ANON) {
                    let mut pmm = asm::physical_memory_manager();
                    pmm.deallocate_fragment(ent.frame);
                }
            }
            AnyFragment::Large(page_prim) => {
                let mut ent = unsafe { unmap_primitive(page_prim)? };
                ent.flush();
                if ent.mapping_flags.contains(EntryMappingFlags::MAP_ANON) {
                    let mut pmm = asm::physical_memory_manager();
                    pmm.deallocate_fragment(ent.frame);
                }
            }
        }
    }

    Ok(())
}
