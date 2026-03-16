//! Bitmap virtual memory manager.

use core::ptr::Alignment;

use crate::arch::{self, VirtAddr};

mod state;

type Container = u64; // Each bit in this type represents the allocation status of a page. Using u64 allows us to track 64 pages per entry in the bitmap.

#[derive(Debug)]
pub struct Bitmap<'a> {
    /// The bitmap data, where each bit represents the allocation status of a page.
    pub data: &'a mut [Container],
    /// The total number of pages managed by this bitmap.
    pub page_count: usize,
    /// The base virtual address that this bitmap manages. This is used to calculate the virtual address corresponding to a given page index in the bitmap.
    pub base: VirtAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BitPtr {
    entry_index: usize, // Index into the bitmap data array
    bit_index: usize,   // Index of the bit within the container (0-63 for u64)
}

impl BitPtr {
    fn new(entry_index: usize, bit_index: usize) -> Self {
        assert!(bit_index < 64); // Ensure bit_index is within bounds for u64
        Self {
            entry_index,
            bit_index,
        }
    }
}

const ENTRY_SIZE: u64 = arch::TABLE_SIZE * 64;

/// The states of the state machine used during allocation.
enum AllocationScanState {
    /// Scanning for a free range of pages that satisfies the allocation request.
    Scanning,
    // Currently allocating a range larger than `ENTRY_SIZE`, which means we are allocating whole entries in the bitmap.
    // this allows for very fast allocation of large ranges of pages, since we can just check if an entry is zero (fully free) or not, and skip over it if it's not.
    // down the line this can be extended to use SIMD instructions to check multiple entries at once for even faster allocation of large ranges.
    // right now for a 2MiB allocation, it would check and set 8 entries in the bitmap, but 2GiB allocations take 4k entries, which is a lot.
    Allocating {
        start: BitPtr,
        len_remaining: usize,
    },
    // We found memory and do not need to continue scanning.
    Found {
        start: BitPtr,
    },
}

struct AllocationStateMachine<'a> {
    size: u64,              // The total size of the allocation request in bytes
    align: u64,             // The alignment requirement for the allocation in bytes
    entry_skip: usize,      // Number of entries to skip for alignment
    page_align: usize,      // Alignment in pages
    entry: u64,             // Current entry being scanned
    entry_ref: &'a mut u64, // Reference to the current entry in the bitmap data
}

impl<'a> Bitmap<'a> {
    /// Initializes the bitmap with the given data slice and page count. The bitmap is cleared to mark all pages as free.
    ///
    /// # Parameters
    /// - `data`: A mutable slice of `u64` values that will be used. The contents do not matter, as they will be cleared during initialization.
    /// - `page_count`: The total number of pages that this bitmap will manage. This determines how many bits in the bitmap are relevant for tracking page allocations.
    ///   For architectures with multiple page sizes, this should be the total number of smallest pages (e.g., 4KB pages) that the bitmap will manage, even if some of those pages may be allocated as larger pages (e.g., 2MB or 1GB pages).
    ///   The bitmap will still track allocations at the granularity of the smallest page size, and larger page allocations will simply mark multiple bits as allocated.
    /// - `base`: The base virtual address that this bitmap manages. This is used to calculate the virtual address corresponding to a given page index in the bitmap.
    ///           `base` must be aligned to the largest page size that this bitmap will manage (e.g., 2MB for x86_64) to ensure that page allocations are properly aligned.
    /// pages. Additionally, the caller must ensure that the bitmap is not concurrently accessed or modified by other parts of the code while it is being initialized, as this could lead to undefined behavior.
    pub unsafe fn init(data: &'a mut [u64], page_count: usize, base: VirtAddr) -> Self {
        // Clear the bitmap to mark all pages as free.
        for entry in data.iter_mut() {
            *entry = 0;
        }
        Self {
            data,
            page_count,
            base,
        }
    }

    fn index_for_addr(&self, addr: VirtAddr) -> usize {
        let page_index = (addr.as_u64() - self.base.as_u64()) / arch::TABLE_SIZE;
        page_index as usize
    }

    fn addr_for_index(&self, index: usize) -> VirtAddr {
        self.base
            .add_checked(index as u64 * arch::TABLE_SIZE)
            .expect("addr_for_index: index overflow")
    }

    fn addr_for_bitptr(&self, bitptr: BitPtr) -> VirtAddr {
        self.addr_for_index(bitptr.entry_index * 64 + bitptr.bit_index)
    }

    fn bitptr_for_addr(&self, addr: VirtAddr) -> BitPtr {
        let index = self.index_for_addr(addr);
        BitPtr::new(index / 64, index % 64)
    }

    /// Allocates a contiguous range of pages with the specified length and alignment.
    /// The length is specified in bytes, and the alignment is also specified in bytes (e.g., 4096 for page-aligned).
    /// The function returns the virtual address of the allocated range if successful, or `None` if there is not enough free space to satisfy the allocation request.
    pub fn alloc(&mut self, len: usize, align: Alignment) -> Option<VirtAddr> {
        todo!()
    }

    /// Frees a previously allocated range of pages starting at the given virtual address and spanning the specified length in bytes.
    ///
    /// # Safety
    /// The caller must ensure that the provided virtual address and length correspond to a valid allocated range of pages that was previously allocated by this bitmap. A
    /// Additionally, the caller must ensure that the range being freed is not currently being accessed or modified by other parts of the code while it is being freed, as this could lead to undefined behavior.
    pub unsafe fn free(&mut self, addr: VirtAddr, len: usize) {}
}

#[cfg(test)]
mod tests {
    use std::alloc::{self, Layout};

    use super::*;

    #[test]
    fn test_bitmap_init() {
        let mut data = [0xFFu64; 4]; // Bitmap to manage 256 pages (4 * 64)
        let page_count = 256;
        let base = VirtAddr::new(0x1000_0000);
        let bitmap = unsafe { Bitmap::init(&mut data, page_count, base) };

        assert_eq!(bitmap.data.len(), 4);
        assert_eq!(bitmap.page_count, 256);
        assert_eq!(bitmap.base, base);
        assert!(bitmap.data.iter().all(|&entry| entry == 0)); // All entries should be cleared
    }

    mod math {
        use super::*;

        #[test]
        fn test_bitmap_addr_index_mapping() {
            let mut data = [0u64; 4]; // Bitmap to manage 256 pages (4 * 64)
            let page_count = 256;
            let base = VirtAddr::new(0x1000_0000);
            let bitmap = unsafe { Bitmap::init(&mut data, page_count, base) };

            // Test index_for_addr and addr_for_index consistency
            for i in 0..page_count {
                let addr = bitmap.addr_for_index(i);
                assert_eq!(bitmap.index_for_addr(addr), i);
            }
        }

        #[test]
        fn test_bitmap_bitptr_conversion() {
            let mut data = [0u64; 4]; // Bitmap to manage 256 pages (4 * 64)
            let page_count = 256;
            let base = VirtAddr::new(0x1000_0000);
            let bitmap = unsafe { Bitmap::init(&mut data, page_count, base) };

            // Test addr_for_bitptr consistency
            for entry_index in 0..bitmap.data.len() {
                for bit_index in 0..64 {
                    let bitptr = BitPtr::new(entry_index, bit_index);
                    let addr = bitmap.addr_for_bitptr(bitptr);
                    let expected_addr = bitmap
                        .base
                        .add_checked((entry_index * 64 + bit_index) as u64 * arch::TABLE_SIZE)
                        .expect("failed to calculate expected addr");
                    assert_eq!(addr, expected_addr);
                }
            }
        }
    }

    pub struct OwnedBitmap<'a> {
        bitmap: Bitmap<'a>,
        container: Box<[u64]>,
    }

    unsafe fn transmute_ref<'a, 'b, T: ?Sized>(r: &'a mut T) -> &'b mut T {
        unsafe { core::mem::transmute(r) }
    }

    impl<'a> OwnedBitmap<'a> {
        pub fn new(cap: usize, base: VirtAddr) -> Self {
            let mut layout = Layout::array::<u64>(cap).expect("failed to create array layout");
            let mut container = vec![0; cap].into_boxed_slice();
            let bitmap =
                unsafe { Bitmap::init(unsafe { transmute_ref(&mut container) }, cap * 64, base) };
            Self { bitmap, container }
        }

        pub fn bitmap(&'a mut self) -> &mut Bitmap<'_> {
            &mut self.bitmap
        }
    }
}
