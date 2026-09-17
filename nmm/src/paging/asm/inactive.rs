use cake::log::error;

use crate::{
    MemError,
    paging::{
        Frame, Large, Page, Small, VirtAddr,
        asm::{self, mounted},
    },
};

pub struct InactiveAddressSpace {
    pub(crate) mut(super) l4_table_frame: Frame<Small>,
    pub(crate) mut(super) scratch_page: Page<Large>,
    pub(crate) mut(super) bootstrap_hhdm_offset: Option<VirtAddr>,
    pub(crate) mut(super) rem: crate::paging::RecursiveEntryManager,
}

impl InactiveAddressSpace {
    // Creates a bootstrap instance of InactiveAddressSpace. This is used during the early boot process before an AddressSpace is loaded.
    pub(crate) unsafe fn bootstrap(
        l4_table_frame: Frame<Small>,
        scratch_page: Page<Large>,
        hhdm_offset: VirtAddr,
    ) -> Self {
        Self {
            l4_table_frame,
            scratch_page,
            bootstrap_hhdm_offset: Some(hhdm_offset),
            rem: crate::paging::RecursiveEntryManager::default(),
        }
    }

    pub fn new() -> Result<Self, crate::paging::MemError> {
        let l4_table_frame = crate::reserve_frame()?;
        unsafe { crate::paging::asm::zero_frame(l4_table_frame)? };
        let asm = crate::paging::asm::active();
        let scratch_page = asm.scratch_page;
        Ok(Self {
            l4_table_frame,
            scratch_page,
            bootstrap_hhdm_offset: None,
            rem: crate::paging::RecursiveEntryManager::default(),
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
        if self.bootstrap_hhdm_offset.is_some() {
            error!(
                "Attempted to mount InactiveAddressSpace during bootstrap phase. This is likely a bug, as the bootstrap address space should not be mounted."
            );
            return Err((crate::paging::MemError::InvalidOperation, self));
        }
        crate::paging::asm::mounted::MountedAddressSpace::try_mount_inactive(self)
    }
}

impl Drop for InactiveAddressSpace {
    fn drop(&mut self) {
        if self.bootstrap_hhdm_offset.is_some() {
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
