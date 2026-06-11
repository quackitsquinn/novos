use core::sync::atomic::{AtomicU8, Ordering};

use cake::Once;

use crate::{arch::VirtAddr, paging::PageTableIndex};

struct AddressSpaceState {
    // 0 = unknown, 1 = HHDM, 2 = recursive
    mode: AtomicU8,
    hhdm_offset: Once<VirtAddr>,
    recursive_index: Once<PageTableIndex>,
}

impl AddressSpaceState {
    pub const fn new() -> Self {
        Self {
            mode: AtomicU8::new(0),
            hhdm_offset: Once::new(),
            recursive_index: Once::new(),
        }
    }

    pub fn cfg_hhdm(&self, offset: VirtAddr) {
        match self
            .mode
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => (),
            Err(1) => panic!("HHDM offset already set"),
            Err(2) => panic!("Address space already configured as recursive"),
            Err(_) => unreachable!(),
        }

        let mut set: bool = false;
        self.hhdm_offset.call_once(|| {
            set = true;
            offset
        });
        if !set {
            panic!("HHDM offset already set");
        }
    }

    pub fn set_recursive(&self, index: PageTableIndex) {
        match self
            .mode
            .compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => (),
            // so this isn't a strict requirement, but currently there's no way to
            // initialize nmm with recursive mapping WITHOUT also setting an initial HHDM offset.
            Err(0) => panic!("Address space not configured as HHDM"),
            Err(2) => panic!("Recursive index already set"),
            Err(_) => unreachable!(),
        }

        let mut set: bool = false;
        self.recursive_index.call_once(|| {
            set = true;
            index
        });
        if !set {
            panic!("Recursive index already set");
        }
    }

    pub fn is_hhdm(&self) -> bool {
        self.mode.load(core::sync::atomic::Ordering::SeqCst) == 1
    }

    pub fn is_recursive(&self) -> bool {
        self.mode.load(core::sync::atomic::Ordering::SeqCst) == 2
    }
}

pub static ADDRESS_SPACE_STATE: AddressSpaceState = AddressSpaceState::new();

/// Sets the HHDM offset for the address space. This should only be called once during initialization, and will panic if called multiple times or if the address space has already been configured as recursive.
pub fn cfg_hhdm(offset: VirtAddr) {
    ADDRESS_SPACE_STATE.cfg_hhdm(offset);
}

/// Enables recursive mapping for the address space by setting the recursive index. This should only be called once during initialization, and will panic if called multiple times or if the address space has not been configured as HHDM.
pub fn enable_recursive(index: PageTableIndex) {
    ADDRESS_SPACE_STATE.set_recursive(index);
}

/// Returns true if the current address space is configured as HHDM-mapped, false otherwise.
pub fn is_hhdm() -> bool {
    ADDRESS_SPACE_STATE.is_hhdm()
}

/// Returns true if the current address space is configured with recursive mapping, false otherwise.
pub fn is_recursive() -> bool {
    ADDRESS_SPACE_STATE.is_recursive()
}

/// Returns the HHDM offset for the address space if it is configured as HHDM-mapped, or None if it is not.
pub fn hhdm_offset() -> Option<VirtAddr> {
    ADDRESS_SPACE_STATE.hhdm_offset.get().copied()
}

/// Returns the recursive index for the address space if it is configured with recursive mapping, or None if it is not.
pub fn recursive_index() -> Option<PageTableIndex> {
    ADDRESS_SPACE_STATE.recursive_index.get().copied()
}
