use core::sync::atomic::AtomicBool;
use spin::{Mutex, MutexGuard};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use super::OnceMutex;

/// A mutex that disables interrupts while it is locked.
pub struct InterruptBlock<T> {
    block: OnceMutex<T>,
    reenable: AtomicBool,
}

impl<T> InterruptBlock<T> {
    pub const fn new(value: T) -> InterruptBlock<T> {
        InterruptBlock {
            block: OnceMutex::new_with(value),
            reenable: AtomicBool::new(false),
        }
    }

    pub const fn uninitialized() -> InterruptBlock<T> {
        InterruptBlock {
            block: OnceMutex::uninitialized(),
            reenable: AtomicBool::new(false),
        }
    }

    pub fn init(&self, value: T) {
        self.block.init(value);
    }

    pub fn lock(&self) -> InterruptGuard<T> {
        let block = self.block.get();
        let was_enabled = x86_64::instructions::interrupts::are_enabled();
        if was_enabled {
            x86_64::instructions::interrupts::disable();
            self.reenable
                .store(true, core::sync::atomic::Ordering::Relaxed);
        } else {
            self.reenable
                .store(false, core::sync::atomic::Ordering::Relaxed);
        }
        InterruptGuard {
            inner: block,
            block: self,
        }
    }
}

/// A guard for an `InterruptBlock`.
/// When this guard is dropped, interrupts are restored to the state before the block was locked.
///
/// It is **undefined behavior** to reenable interrupts while the block is still locked.
pub struct InterruptGuard<'a, T> {
    inner: MutexGuard<'a, T>,
    block: &'a InterruptBlock<T>,
}

impl<'a, T> core::ops::Deref for InterruptGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> core::ops::DerefMut for InterruptGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Drop for InterruptGuard<'a, T> {
    fn drop(&mut self) {
        if self
            .block
            .reenable
            .load(core::sync::atomic::Ordering::Relaxed)
        {
            x86_64::instructions::interrupts::enable();
        }
    }
}
