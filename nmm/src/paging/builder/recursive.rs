use core::sync::atomic::AtomicBool;

use crate::{
    arch::RecursivePageTable,
    paging::{AddressExt, PageTable, asm},
};

pub struct RecursiveAddressSpaceBuilder<'a> {
    table: RecursivePageTable<'a>,
}

// TODO: needs to be core local when mp supported, since each core will have it's own pair of recursive entries.
static RECURSIVE_LOCKED: AtomicBool = AtomicBool::new(false);

impl<'a> RecursiveAddressSpaceBuilder<'a> {
    pub fn acquire_instance() -> Self {
        if RECURSIVE_LOCKED.swap(true, core::sync::atomic::Ordering::Acquire) {
            panic!("RecursiveAddressSpaceBuilder instance already acquired");
        }

        let active_as = asm::active();
        Self {
            table: unsafe {
                RecursivePageTable::new(
                    &mut *active_as
                        .l4_table()
                        .start_address()
                        .as_mut_ptr::<PageTable>(),
                    crate::arch::RECURSIVE_SLOT1,
                )
            },
        }
    }
}
