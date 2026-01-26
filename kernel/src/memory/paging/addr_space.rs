//! Per address space information and utilities.

use x86_64::{
    PhysAddr,
    structures::paging::{FrameAllocator, Mapper, PageTableFlags, Translate},
};

use crate::memory::{
    self,
    paging::{
        ACTIVE_PAGE_TABLE,
        map::{
            ADDRESS_SPACE_INFO_END_RAW, ADDRESS_SPACE_INFO_START, ADDRESS_SPACE_INFO_START_PAGE,
        },
        phys::FRAME_ALLOCATOR,
    },
};

/// Contains information about the current address space.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AddressSpaceInfo {
    /// The CR3 value of the address space.
    pub cr3: PhysAddr,
}

impl AddressSpaceInfo {
    /// Writes the address space info to the active page table.
    pub unsafe fn write_to_active(&self) -> Result<(), ()> {
        let mut active_page_table = ACTIVE_PAGE_TABLE.write();
        let mut frame_allocator = FRAME_ALLOCATOR.get();
        unsafe {
            active_page_table
                .map_to(
                    ADDRESS_SPACE_INFO_START_PAGE,
                    frame_allocator
                        .allocate_frame()
                        .expect("failed to allocate frame for address space info"),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    &mut *frame_allocator,
                )
                .map_err(|_| ())?
                .flush();
        }
        let dst_ptr = ADDRESS_SPACE_INFO_START.as_mut_ptr::<AddressSpaceInfo>();
        unsafe {
            dst_ptr.write(*self);
        }
        Ok(())
    }
}

/// Reads the address space info from the active page table.
pub fn read() -> AddressSpaceInfo {
    if !memory::is_initialized() {
        panic!("Attempted to read AddressSpaceInfo before memory was initialized");
    }

    // SAFETY: It is the kernel's responsibility to always have a valid AddressSpaceInfo mapped at the
    // specified location. (unless memory is uninitialized, which we check for above)
    unsafe { *(ADDRESS_SPACE_INFO_START.as_u64() as *const AddressSpaceInfo) }
}
