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
mod test_util {
    //! This module provides a few utility types and functions for host testing.
    use core::mem::transmute;
    use std::alloc::{Layout, alloc, dealloc};

    use x86_64::{PhysAddr, VirtAddr};

    use crate::*;

    pub struct DummyPageAllocator {
        pages: Vec<(KernelPage, KernelPhysFrame)>,
    }

    impl DummyPageAllocator {
        pub fn new() -> Self {
            DummyPageAllocator { pages: Vec::new() }
        }

        pub fn used_pages(&self) -> &[(KernelPage, KernelPhysFrame)] {
            &self.pages
        }
    }

    impl Iterator for DummyPageAllocator {
        type Item = (KernelPage, KernelPhysFrame);

        fn next(&mut self) -> Option<Self::Item> {
            let res = unsafe { alloc(Layout::from_size_align(4096, 4096).unwrap()) };
            if res.is_null() {
                panic!("Failed to allocate page");
            }

            let page = unsafe { transmute::<_, KernelPage>(res) };
            let frame = KernelPhysFrame::from_start_address(PhysAddr::new(res as u64)).unwrap();
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

    #[test]
    fn test_dummy_page_allocator_valid_memory() {
        let mut allocator = DummyPageAllocator::new();
        let (page, frame) = allocator.next().expect("Failed to allocate page");
        assert!(page.start_address().as_u64() != 0);
        assert!(frame.start_address().as_u64() != 0);
        unsafe { page.start_address().as_mut_ptr::<()>().write_bytes(0, 4096) };
    }
}
