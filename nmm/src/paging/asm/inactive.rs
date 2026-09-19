use cake::log::error;

use crate::{
    MapFlags, MemError,
    paging::{
        AddressExt, Frame, Large, MemoryFragment, Page, PageTable, PageTableEntry, PageTableIndex,
        RecursiveEntryManager, Small,
        asm::{self, mounted},
    },
};

pub struct InactiveAddressSpace {
    pub(crate) mut(super) l4_table_frame: Frame<Small>,
    pub(crate) mut(super) scratch_page: Page<Large>,
    pub(crate) mut(super) is_bootstrap: bool,
    pub(crate) mut(super) recursive_index: PageTableIndex,
    pub(crate) mut(super) rem: crate::paging::RecursiveEntryManager,
}

impl InactiveAddressSpace {
    // Creates a bootstrap instance of InactiveAddressSpace. This is used during the early boot process before an AddressSpace is loaded.
    pub(crate) unsafe fn bootstrap(
        l4_table_frame: Frame<Small>,
        scratch_page: Page<Large>,
        recursive_index: PageTableIndex,
    ) -> Self {
        let mut rem = RecursiveEntryManager::default();
        if rem.manages(recursive_index) {
            unsafe {
                rem.set_entry(recursive_index, true);
            }
        }
        Self {
            l4_table_frame,
            scratch_page,
            is_bootstrap: true,
            recursive_index,
            rem: crate::paging::RecursiveEntryManager::default(),
        }
    }

    pub fn new(recursive_index: PageTableIndex) -> Result<Self, crate::paging::MemError> {
        let l4_table_frame = crate::reserve_frame()?;
        let asm = crate::paging::asm::active();
        let scratch_page = asm.scratch_page;
        let rem = RecursiveEntryManager::default();
        unsafe {
            asm::map_with_scratch_page(l4_table_frame, MapFlags::WRITABLE, |s| {
                let l4_table = &mut *s.start_address().as_mut_ptr::<PageTable>();
                l4_table.clear();
                l4_table.set_entry(
                    recursive_index,
                    PageTableEntry::new(l4_table_frame, MapFlags::WRITABLE),
                );
            })?;
        };
        Ok(Self {
            l4_table_frame,
            scratch_page,
            is_bootstrap: false,
            recursive_index,
            rem,
        })
    }

    /// Activates this inactive address space, making it the currently active address space for the CPU.
    /// This will consume this InactiveAddressSpace, and it will no longer be usable after this call.
    pub unsafe fn activate(self) -> Result<(), MemError> {
        asm::activate_inactive_space(self)
    }

    /// Attempts to mount this inactive address space into the current active address space, allowing for direct access to its page tables.
    /// This will consume this InactiveAddressSpace, and it will no longer be usable after this call (unless the mount operation fails).
    pub fn try_mount(
        self,
    ) -> Result<crate::paging::asm::mounted::MountedAddressSpace, (crate::paging::MemError, Self)>
    {
        crate::paging::asm::mounted::MountedAddressSpace::try_mount_inactive(self)
    }
}

impl Drop for InactiveAddressSpace {
    fn drop(&mut self) {
        if self.is_bootstrap {
            error!(
                "Dropping InactiveAddressSpace during bootstrap phase. This is likely a bug, as the bootstrap address space should not be dropped."
            );
            return;
        }

        if let Ok(mas) = unsafe { mounted::mount(self) } {
            if let Err(e) = mas.free() {
                error!(
                    "Failed to free InactiveAddressSpace during drop: {:?}. This may indicate a memory leak.",
                    e
                );
            }
        } else {
            error!(
                "Failed to mount InactiveAddressSpace during drop. This may indicate a memory leak."
            );
        }
    }
}
