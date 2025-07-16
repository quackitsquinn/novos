#![cfg_attr(not(test), no_std)]

use x86_64::structures::paging::{Page, PhysFrame, Size4KiB};

#[cfg(any(feature = "alloc", test))]
extern crate alloc;

pub mod pagetable;
pub mod phys;
#[cfg(any(feature = "alloc", test))]
pub mod virt;

pub type KernelPageSize = Size4KiB;
pub type KernelPage = Page<KernelPageSize>;
pub type KernelPhysFrame = PhysFrame<KernelPageSize>;

#[cfg(test)]
mod tests {
    use std::alloc::{Layout, alloc, dealloc};

    use x86_64::{PhysAddr, VirtAddr};

    use super::*;

    pub struct DummyPageAllocator {
        pages: Vec<(KernelPage, KernelPhysFrame)>,
        index: usize,
    }

    impl DummyPageAllocator {
        pub fn new() -> Self {
            DummyPageAllocator {
                pages: Vec::new(),
                index: 0,
            }
        }
    }

    impl Iterator for DummyPageAllocator {
        type Item = (KernelPage, KernelPhysFrame);

        fn next(&mut self) -> Option<Self::Item> {
            let res = unsafe { alloc(Layout::from_size_align(4096, 4096).unwrap()) };
            if res.is_null() {
                panic!("Failed to allocate page");
            }

            let page = KernelPage::from_start_address(VirtAddr::new(res as u64)).unwrap();
            let frame =
                KernelPhysFrame::from_start_address(PhysAddr::new(self.index as u64)).unwrap();
            self.index += 4096;
            self.pages.push((page, frame));
            Some((page, frame))
        }
    }

    impl Drop for DummyPageAllocator {
        fn drop(&mut self) {
            for (page, _) in &self.pages {
                unsafe {
                    dealloc(
                        page.start_address().as_mut_ptr(),
                        Layout::from_size_align(4096, 4096).unwrap(),
                    );
                }
            }
        }
    }
}
