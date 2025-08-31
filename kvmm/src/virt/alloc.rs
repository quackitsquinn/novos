use crate::KernelPage;
use bitvec::{BitArr, array::BitArray, order::Msb0};
use cake::error;
use x86_64::structures::paging::{Page, page::PageRange};

pub struct SimplePageAllocator {
    range: PageRange,
    cap: usize,
    bits: BitArr!(for 0x1000, in u16, Msb0),
    #[cfg(test)]
    pub realloc_fallback: bool,
}

impl SimplePageAllocator {
    /// Creates a new `SimplePageAllocator` that manages a range of pages starting from `root` and spanning `page_count` pages.
    ///
    /// # Safety
    /// The caller must ensure that the memory range defined by `root` and `page_count` is valid and not used by any other part of the system.
    /// The `page_count` must not exceed 0x1000 (4096 pages).
    pub unsafe fn new(root: Page, page_count: usize) -> Self {
        assert!(page_count <= 0x1000);
        let range = Page::range(root, root + page_count as u64);
        SimplePageAllocator {
            range,
            cap: page_count,
            bits: BitArray::ZERO,
            #[cfg(test)]
            realloc_fallback: false,
        }
    }

    /// Allocates pages of memory. Returns a pointer to the start of the allocated memory, or null if allocation fails.
    ///
    /// **`pagecount` is in units of 4KiB pages.**
    ///
    /// # Safety
    /// The caller must ensure that the allocated memory is properly deallocated using `dealloc` when no longer needed.
    pub unsafe fn alloc(&mut self, pagecount: usize) -> *mut u8 {
        if let Some(start) = self.find_sequential_zeros(pagecount) {
            for i in start..start + pagecount {
                self.bits.set(i, true);
            }
            let page = self.range.start + start as u64;
            return page.start_address().as_mut_ptr();
        }
        error!("alloc failed: out of memory");
        core::ptr::null_mut()
    }

    /// Deallocates previously allocated pages of memory.
    /// # Safety
    /// The caller must ensure that the pointer was previously allocated by `alloc` and that the
    /// `pagecount` matches the original allocation.
    /// If `pagecount` is incorrect in any way, the behavior is undefined and will likely lead to memory corruption.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, pagecount: usize) {
        let index = self.get_index_from_ptr(ptr);
        if index.is_none() {
            error!("dealloc failed: pointer {:p} is out of range", ptr);
            return;
        }
        let index = index.unwrap();
        for i in index as usize..index as usize + pagecount {
            self.bits.set(i, false);
        }
    }

    pub unsafe fn realloc(
        &mut self,
        ptr: *mut u8,
        old_pagecount: usize,
        new_pagecount: usize,
    ) -> *mut u8 {
        if new_pagecount == old_pagecount {
            return ptr;
        }
        let index = self.get_index_from_ptr(ptr);
        if index.is_none() {
            error!("realloc failed: pointer {:p} is out of range", ptr);
            return core::ptr::null_mut();
        }
        let index = index.unwrap();
        let index_end = index + old_pagecount;
        let new_index_end = index + new_pagecount;

        if old_pagecount > new_pagecount {
            for i in new_index_end..index_end {
                self.bits.set(i, false);
            }
            return ptr;
        }

        // Try to expand in place
        let mut can_expand = true;
        for bit in self.bits[index_end..new_index_end].iter() {
            if *bit {
                // Can't expand in place
                can_expand = false;
                break;
            }
        }

        if can_expand {
            for i in index_end..new_index_end {
                self.bits.set(i, true);
            }
            return ptr;
        }

        #[cfg(test)]
        {
            self.realloc_fallback = true;
            // Avoid any memory manipulation in tests
            return core::ptr::null_mut();
        }

        // Fallback to bog standard alloc + copy + free
        // TODO: Somehow allow for in-place reallocs that move the memory block (e.g. moving the block back a bit)
        // Would also help reduce fragmentation
        #[allow(unreachable_code)]
        let new_ptr = unsafe { self.alloc(new_pagecount) };
        if !new_ptr.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ptr,
                    new_ptr,
                    new_pagecount * KernelPage::SIZE as usize,
                );
                self.dealloc(ptr, old_pagecount)
            };
        }
        new_ptr
    }

    fn find_sequential_zeros(&mut self, count: usize) -> Option<usize> {
        if count == 1 {
            let pos = self.bits.first_zero()?;
            if pos > self.cap - count {
                return None;
            }
            return Some(pos);
        }

        let mut current_run = 0;
        let mut start_index = 0;

        for (i, bit) in self.bits.iter().enumerate() {
            if i > self.cap - (count - current_run) {
                return None;
            }
            if !*bit {
                if current_run == 0 {
                    start_index = i;
                }
                current_run += 1;
                if current_run == count {
                    return Some(start_index);
                }
            } else {
                current_run = 0;
            }
        }
        None
    }

    fn get_index_from_ptr(&self, ptr: *mut u8) -> Option<usize> {
        let page = KernelPage::containing_address(x86_64::VirtAddr::new(ptr as u64));
        let index =
            (page.start_address().as_u64() - self.range.start.start_address().as_u64()) / 0x1000;
        if index >= self.cap as u64 {
            return None;
        }
        Some(index as usize)
    }
}

/// Converts a `usize` size in bytes to the number of 4KiB pages required to satisfy the size.
pub fn usize_to_pages(size: usize) -> usize {
    let page_size = 4096;
    let pages = (size + page_size - 1) / page_size;
    pages
}

/// Converts a `core::alloc::Layout` to the number of 4KiB pages required to satisfy the layout.
///
/// This does not account for alignment above 4KiB.
#[cfg(any(feature = "alloc", test))]
pub fn layout_to_pages(layout: core::alloc::Layout) -> usize {
    let size = layout.size();
    let pages = usize_to_pages(size);
    pages
}

#[cfg(test)]
mod tests {
    use x86_64::structures::paging::PageSize;

    use super::*;

    fn new(start: usize, page_count: usize) -> SimplePageAllocator {
        unsafe {
            SimplePageAllocator::new(
                Page::from_start_address(x86_64::VirtAddr::new(start as u64)).unwrap(),
                page_count,
            )
        }
    }

    #[test]
    fn test_new() {
        let allocator = new(0x1000, 10);
        assert_eq!(allocator.cap, 10);
        assert_eq!(allocator.bits.count_ones(), 0);
    }

    #[test]
    fn test_alloc() {
        let mut allocator = new(0x1000, 10);
        let ptr = unsafe { allocator.alloc(3) };
        assert_eq!(ptr as usize, 0x1000);
        assert_eq!(allocator.bits.count_ones(), 3);
        assert!(allocator.bits[0..3].all());
        assert!(allocator.bits[3..].not_any());
        let ptr2 = unsafe { allocator.alloc(2) };
        assert_eq!(ptr2 as usize, 0x4000);
        assert_eq!(allocator.bits.count_ones(), 5);
        assert!(allocator.bits[0..5].all());
        assert!(allocator.bits[5..].not_any());
    }

    #[test]
    fn test_alloc_fail() {
        let mut allocator = new(0x1000, 5);
        let ptr = unsafe { allocator.alloc(5) };
        assert!(!ptr.is_null());
        assert_eq!(allocator.bits.count_ones(), 5);
        let ptr = unsafe { allocator.alloc(1) };
        assert!(ptr.is_null());
        assert_eq!(allocator.bits.count_ones(), 5); // All pages are allocated
    }

    #[test]
    fn test_dealloc() {
        let mut allocator = new(0x1000, 10);
        let ptr = unsafe { allocator.alloc(4) };
        assert!(!ptr.is_null());
        assert_eq!(allocator.bits.count_ones(), 4);
        unsafe { allocator.dealloc(ptr, 4) };
        assert_eq!(allocator.bits.count_ones(), 0);
        assert!(allocator.bits.not_any());
    }

    #[test]
    fn test_realloc_expand_in_place() {
        let mut allocator = new(0x1000, 10);
        let ptr = unsafe { allocator.alloc(3) };
        let ptr2 = unsafe { allocator.alloc(2) };
        let ptr3 = unsafe { allocator.alloc(3) };
        assert!(!ptr.is_null());
        assert!(!ptr2.is_null());
        assert!(!ptr3.is_null());
        unsafe {
            allocator.dealloc(ptr2, 2);
        }
        let ptr_realloc = unsafe { allocator.realloc(ptr, 3, 5) };
        assert_eq!(ptr_realloc, ptr);
        assert_eq!(allocator.bits.count_ones(), 8);
        assert!(allocator.bits[0..8].all());
        assert!(allocator.bits[8..].not_any());
    }

    #[test]
    fn test_realloc_shrink_in_place() {
        let mut allocator = new(0x1000, 10);
        let ptr = unsafe { allocator.alloc(5) };
        assert!(!ptr.is_null());
        let ptr_realloc = unsafe { allocator.realloc(ptr, 5, 3) };
        assert_eq!(ptr_realloc, ptr);
        assert_eq!(allocator.bits.count_ones(), 3);
        assert!(allocator.bits[0..3].all());
        assert!(allocator.bits[3..].not_any());
    }

    #[test]
    fn test_realloc_fallback() {
        let mut allocator = new(0x1000, 10);
        let ptr = unsafe { allocator.alloc(3) };
        let ptr2 = unsafe { allocator.alloc(3) };
        assert!(!ptr.is_null());
        assert!(!ptr2.is_null());
        let ptr_realloc = unsafe { allocator.realloc(ptr, 3, 5) };
        assert!(ptr_realloc.is_null());
        assert!(allocator.realloc_fallback);
    }

    #[test]
    fn test_layout_to_pages() {
        for i in 1..4097 {
            let layout = core::alloc::Layout::from_size_align(i, 1).unwrap();
            let pages = layout_to_pages(layout);
            assert_eq!(pages, 1);
        }

        for i in 4097..8193 {
            let layout = core::alloc::Layout::from_size_align(i, 1).unwrap();
            let pages = layout_to_pages(layout);
            assert_eq!(pages, 2);
        }
    }
}
