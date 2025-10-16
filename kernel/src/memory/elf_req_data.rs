//! Post page table switch data structures and abstractions.
use core::{alloc::Layout, fmt::Debug};

use alloc::alloc::alloc;
use cake::spin::{Mutex, Once};
use cake::{LimineData, limine::response::ExecutableFileResponse, spin};
use kelp::Elf;

/// The kernel's ELF executable, stored in a way that allows moving it to the heap later.
pub struct KernelElf {
    elf: Mutex<Elf<'static>>,
    limine_data: Mutex<Option<&'static [u8]>>,
    kernel_data: Once<&'static [u8]>,
}

impl KernelElf {
    /// Creates a new `KernelElf` from the given Limine executable file request.
    pub fn new(limine_data: LimineData<'_, ExecutableFileResponse>) -> Self {
        let limine_data = unsafe {
            core::slice::from_raw_parts(
                limine_data.file().addr(),
                limine_data.file().size() as usize,
            )
        };
        let elf = Elf::new(limine_data).expect("Limine executable is not a valid ELF file");
        let kernel_data = Once::new();
        KernelElf {
            elf: Mutex::new(elf),
            limine_data: Mutex::new(Some(limine_data)),
            kernel_data,
        }
    }

    /// Returns the Elf instance for the kernel. This will panic if the requests have terminated and the ELF data has not been copied to the heap yet.
    pub fn elf(&self) -> spin::MutexGuard<'_, Elf<'static>> {
        if !self.kernel_data.is_completed() {
            panic!("Kernel ELF data has not been copied to the heap yet");
        }
        self.elf.lock()
    }

    /// Returns the Elf instance for the kernel.
    ///
    /// # Safety
    ///
    /// The caller must ensure the returned reference is discarded before limine requests are terminated.
    /// It is *not* undefined behavior to call this function before it has been copied.
    pub unsafe fn elf_unchecked(&self) -> spin::MutexGuard<'_, Elf<'static>> {
        self.elf.lock()
    }

    /// Copies the ELF data to the heap.
    pub fn copy_to_heap(&self) {
        let mut limine_data = self.limine_data.lock();
        if let Some(data) = limine_data.take() {
            let ptr = unsafe { alloc(Layout::from_size_align(data.len(), 1).unwrap()) };
            if ptr.is_null() {
                panic!("Failed to allocate memory for kernel ELF");
            }
            unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len()) };
            let slice = unsafe { core::slice::from_raw_parts(ptr, data.len()) };
            self.kernel_data.call_once(|| slice);
            let elf = Elf::new(slice).expect("Heap ELF is not a valid ELF file");
            *self.elf.lock() = elf;
        }
    }
}

impl Debug for KernelElf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelElf").field("elf", &self.elf).finish()
    }
}
