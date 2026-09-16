use crate::{
    MemError,
    bitmap::VirtualMemoryManager,
    paging::{Frame, Large, Page, Small, VirtAddr, asm},
};

pub struct InactiveAddressSpace {
    pub(crate) mut(super) l4_table_frame: Frame<Small>,
    pub(crate) mut(super) scratch_page: Page<Large>,
    pub(crate) mut(super) bootstrap_hhdm_offset: Option<VirtAddr>,
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
        })
    }

    /// Activates this inactive address space, making it the currently active address space for the CPU.
    /// This will consume this InactiveAddressSpace, and it will no longer be usable after this call.
    pub unsafe fn activate(self) -> Result<(), MemError> {
        asm::activate_inactive_space(self)
    }
}

impl Drop for InactiveAddressSpace {
    fn drop(&mut self) {
        if self.bootstrap_hhdm_offset.is_some() {
            return;
        }
        todo!()
    }
}
