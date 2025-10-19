use core::mem;

use cake::{
    Once, RawRwLock, RwLockReadGuard, RwLockWriteGuard,
    lock_api::{MappedRwLockReadGuard, MappedRwLockWriteGuard},
};
use x86_64::structures::idt::InterruptDescriptorTable;

use crate::{
    interrupts::without_interrupts,
    mp::{CloneBootstrap, CoreLocal},
};

/// The template IDT used to initialize local IDTs.
pub static LOCAL_IDT_TEMPLATE: Once<InterruptDescriptorTable> = Once::new();

/// A structure that holds the local IDT for each core.
#[derive(Debug)]
pub struct LocalIdt {
    tables: CoreLocal<
        (InterruptDescriptorTable, InterruptDescriptorTable),
        CloneBootstrap<(InterruptDescriptorTable, InterruptDescriptorTable)>,
    >,
}

impl LocalIdt {
    /// Create a new LocalIdt structure.
    pub const fn new() -> Self {
        LocalIdt {
            tables: CoreLocal::new(
                (
                    InterruptDescriptorTable::new(),
                    InterruptDescriptorTable::new(),
                ),
                CloneBootstrap::new(),
            ),
        }
    }

    /// Get a read-only reference to the local IDT.
    pub fn get(&self) -> MappedRwLockReadGuard<'_, RawRwLock, InterruptDescriptorTable> {
        RwLockReadGuard::map(self.tables.read(), |(table, _)| table)
    }

    /// Get a mutable reference to the local IDT.
    /// Any modifications will not be visible until `swap` is called.
    pub fn get_mut(&self) -> MappedRwLockWriteGuard<'_, RawRwLock, InterruptDescriptorTable> {
        RwLockWriteGuard::map(self.tables.write(), |(_, table)| table)
    }

    /// Swap the front and back IDT tables.
    pub fn swap(&self) {
        let mut tables = self.tables.write();
        without_interrupts(|| {
            let (front, back) = &mut *tables;
            mem::swap(front, back);
        });
    }

    /// Sync the back table to match the front table.
    pub fn sync(&self) {
        let mut tables = self.tables.write();
        without_interrupts(|| {
            let (front, back) = &mut *tables;
            *back = front.clone();
        });
    }

    /// Combines both swap and sync: swaps the tables and then syncs the back to match the front.
    pub fn swap_and_sync(&self) {
        let mut tables = self.tables.write();
        without_interrupts(|| {
            let (front, back) = &mut *tables;
            mem::swap(front, back);
            *back = front.clone();
        });
    }

    /// Load the local IDT into the CPU's IDT register. This only needs to be done once per core.
    ///
    /// # Safety
    /// The caller must ensure that no interrupts occur while the IDT is being modified.
    pub unsafe fn load(&'static self) {
        let tables = self.tables.read();
        // Safety: Any modification to the IDT is always done with interrupts disabled.
        // Thus, converting the read guard to a static reference is safe.
        let (front, _) =
            unsafe { &*(&*tables as *const (InterruptDescriptorTable, InterruptDescriptorTable)) };
        front.load();
    }
}
