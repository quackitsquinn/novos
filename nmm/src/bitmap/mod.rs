//! Bitmap virtual memory manager.

use core::fmt::Debug;
use core::ops::{Index, IndexMut};
use core::ptr::Alignment;

use crate::bitmap::state::range_mask;
use crate::{
    arch::{self, VirtAddr},
    bitmap::state::AllocationStateMachine,
};

mod state;

type Container = u64; // Each bit in this type represents the allocation status of a page. Using u64 allows us to track 64 pages per entry in the bitmap.

/// A bitmap-based virtual memory manager that tracks the allocation status of pages using a bitmap data structure.
/// Each bit in the bitmap represents whether a corresponding page is allocated or free.
pub struct Bitmap<'a> {
    /// The bitmap data, where each bit represents the allocation status of a page.
    pub data: &'a mut [Container],
    /// The total number of pages managed by this bitmap.
    pub page_count: u64,
    /// The base virtual address that this bitmap manages. This is used to calculate the virtual address corresponding to a given page index in the bitmap.
    pub base: VirtAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BitPtr {
    entry_index: u64, // Index into the bitmap data array
    bit_index: u8,    // Index of the bit within the container (0-63 for u64)
}

impl BitPtr {
    fn new(entry_index: u64, bit_index: u8) -> Self {
        assert!(bit_index < 64); // Ensure bit_index is within bounds for u64
        Self {
            entry_index,
            bit_index,
        }
    }

    /// Skips to the next entry in the bitmap, returning the new BitPtr and the number of bits skipped (which is 64 - bit_index).
    fn skip_entry(self) -> (Self, u64) {
        (
            Self::new(self.entry_index + 1, 0),
            64 - self.bit_index as u64,
        )
    }
}

const ENTRY_SIZE: u64 = arch::TABLE_SIZE * 64;

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
    pub unsafe fn init(data: &'a mut [u64], page_count: u64, base: VirtAddr) -> Self {
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

    fn addr_for_index(&self, index: u64) -> VirtAddr {
        self.base
            .add_checked(index as u64 * arch::TABLE_SIZE)
            .expect("addr_for_index: index overflow")
    }

    fn addr_for_bitptr(&self, bitptr: BitPtr) -> VirtAddr {
        self.addr_for_index(bitptr.entry_index * 64 + bitptr.bit_index as u64)
    }

    fn bitptr_for_addr(&self, addr: VirtAddr) -> Option<BitPtr> {
        let offset = addr.sub_checked(self.base.as_u64())?;
        let page_index = offset.as_u64() / arch::TABLE_SIZE;
        if page_index >= self.page_count as u64 {
            return None;
        }
        Some(BitPtr::new(
            (page_index / 64) as u64,
            (page_index % 64) as u8,
        ))
    }

    /// Marks n bits starting at bitptr as allocated in the bitmap.
    ///
    /// n can be greater than 64.
    unsafe fn set(&mut self, bitptr: BitPtr, n: u64) {
        // TODO: SIMD optimizations
        if n > 64 {
            let (bitptr, remaining) = bitptr.skip_entry();
            unsafe { self.set(bitptr, n - remaining) };
        }
        let mask = range_mask(n);
        self[bitptr.entry_index as usize] |= mask << bitptr.bit_index;
    }

    /// Marks n bits starting at bitptr as free in the bitmap.
    unsafe fn clear(&mut self, bitptr: BitPtr, n: u64) {
        // TODO: SIMD optimizations
        if n > 64 {
            let (bitptr, remaining) = bitptr.skip_entry();
            unsafe { self.clear(bitptr, n - remaining) };
        }
        let mask = range_mask(n);
        self[bitptr.entry_index as usize] &= !(mask << bitptr.bit_index);
    }
    // Some(bool) == is/isn't allocated, None == out of bounds
    #[allow(unused)]
    pub(crate) fn is_set(&self, bitptr: BitPtr, n: u64) -> Option<bool> {
        if bitptr.entry_index >= self.data.len() as u64 {
            return None; // Out of bounds
        }

        if n > 64 {
            let (bitptr, remaining) = bitptr.skip_entry();
            return self.is_set(bitptr, n - remaining);
        }
        let mask = range_mask(n);
        let entry_shf = self[bitptr.entry_index as usize] >> bitptr.bit_index;
        Some((entry_shf & mask) == mask)
    }

    /// Allocates a contiguous range of pages with the specified length and alignment.
    /// The length is specified in bytes, and the alignment is also specified in bytes (e.g., 4096 for page-aligned).
    /// The function returns the virtual address of the allocated range if successful, or `None` if there is not enough free space to satisfy the allocation request.
    pub fn alloc(&'a mut self, len: usize, align: Alignment) -> Option<VirtAddr> {
        let mut state_machine = AllocationStateMachine::new(len as u64, align, self);
        state_machine.run()
    }

    /// Frees a previously allocated range of pages starting at the given virtual address and spanning the specified length in bytes.
    ///
    /// # Safety
    /// The caller must ensure that the provided virtual address and length correspond to a valid allocated range of pages that was previously allocated by this bitmap. A
    /// Additionally, the caller must ensure that the range being freed is not currently being accessed or modified by other parts of the code while it is being freed, as this could lead to undefined behavior.
    pub unsafe fn free(&mut self, addr: VirtAddr, len: usize) {
        let bitptr = match self.bitptr_for_addr(addr) {
            Some(bitptr) => bitptr,
            None => panic!("free: address out of bounds"),
        };
        unsafe { self.clear(bitptr, len as u64) };
    }
}

impl Debug for Bitmap<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Bitmap [")?;
        let allocated = self
            .data
            .iter()
            .fold(0, |i, entry| i + entry.count_ones() as u64);
        let free = self.page_count - allocated;
        let utilization = allocated as f64 / self.page_count as f64 * 100.0;
        write!(
            f,
            "allocated: {}, free: {}, {} utilization]",
            allocated, free, utilization
        )
    }
}

impl Index<usize> for Bitmap<'_> {
    type Output = Container;

    fn index(&self, index: usize) -> &<Self as Index<usize>>::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for Bitmap<'_> {
    fn index_mut(&mut self, index: usize) -> &mut <Self as Index<usize>>::Output {
        &mut self.data[index]
    }
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
                let expected_addr = base
                    .add_checked(i as u64 * arch::TABLE_SIZE)
                    .expect("failed to calculate expected addr");
                assert_eq!(addr, expected_addr);
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
                    let bitptr = BitPtr::new(entry_index as u64, bit_index);
                    let addr = bitmap.addr_for_bitptr(bitptr);
                    let expected_addr = bitmap
                        .base
                        .add_checked(
                            (entry_index * 64 + bit_index as usize) as u64 * arch::TABLE_SIZE,
                        )
                        .expect("failed to calculate expected addr");
                    assert_eq!(addr, expected_addr);
                }
            }
        }

        #[test]
        fn test_bitptr_skip_entry() {
            let bitptr = BitPtr::new(0, 10); // Start at the 10th bit of the first entry
            let (new_bitptr, skipped) = bitptr.skip_entry();
            assert_eq!(new_bitptr.entry_index, 1); // Should move to the next entry
            assert_eq!(new_bitptr.bit_index, 0); // Should reset bit index to 0
            assert_eq!(skipped, 54); // Should skip 54 bits (from bit index 10 to the end of the entry)
        }

        #[test]
        fn test_bitmap_set() {
            let mut data = [0u64; 4]; // Bitmap to manage 256 pages (4 * 64)
            let page_count = 256;
            let base = VirtAddr::new(0x1000_0000);
            let mut bitmap = unsafe { Bitmap::init(&mut data, page_count, base) };

            let bitptr = BitPtr::new(0, 0); // Start at the first bit
            unsafe { bitmap.set(bitptr, 70) }; // This should set bits 0-69 (crossing into the second entry)
            assert_eq!(bitmap[0], u64::MAX); // First entry should be fully set
            assert_eq!(bitmap[1], (1u64 << 6) - 1); // Second entry should have the first 6 bits set

            bitmap.data.fill(0); // Reset the bitmap

            let bitptr = BitPtr::new(2, 10); // Start at the 138th bit
            unsafe { bitmap.set(bitptr, 10) }; // This should set bits 138-147
            assert_eq!(bitmap[2], (0b1111111111u64) << 10); // Bits 10-19 of the third entry should be set

            bitmap.data.fill(0); // Reset the bitmap

            let bitptr = BitPtr::new(0, 60); // Start at the 60th bit
            unsafe { bitmap.set(bitptr, 70) }; // This should set bits 60
            assert_eq!(bitmap[0], (0b1111u64) << 60); // Bits 60-63 of the first entry should be set
            assert_eq!(bitmap[1], u64::MAX, "{:b}", bitmap[1]); // All bits of the second entry should be set
            assert_eq!(bitmap[2], (1u64 << 2) - 1); // Bits 0-2 of the third entry should be set
        }

        #[test]
        fn test_bitmap_clear() {
            let mut container = OwnedBitmap::new(4, VirtAddr::new(0x1000_0000)); // Bitmap to manage 256 pages (4 * 64)
            let bitmap = container.bitmap();
            let bitptr = BitPtr::new(0, 0); // Start at the first bit
            unsafe { bitmap.set(bitptr, 70) }; // Set bits 0-69
            unsafe { bitmap.clear(bitptr, 70) }; // Clear bits 0-69
            assert_eq!(bitmap[0], 0); // First entry should be cleared
            assert_eq!(bitmap[1], 0); // Second entry should be cleared
        }

        #[test]
        fn test_bitmap_is_set() {
            let mut container = OwnedBitmap::new(4, VirtAddr::new(0x1000_0000)); // Bitmap to manage 256 pages (4 * 64)
            let bitmap = container.bitmap();
            let bitptr = BitPtr::new(0, 0); // Start at the first bit
            unsafe { bitmap.set(bitptr, 70) }; // Set bits 0-69

            assert_eq!(bitmap.is_set(bitptr, 70), Some(true)); // Bits 0-69 should be set
            assert_eq!(bitmap.is_set(BitPtr::new(1, 6), 10), Some(false)); // Bits 70-79 should not be set
            assert_eq!(bitmap.is_set(BitPtr::new(3, 0), 64), Some(false)); // Bits 192-255 should not be set
            assert_eq!(bitmap.is_set(BitPtr::new(4, 0), 1), None); // Out of bounds
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
            let bitmap = unsafe {
                Bitmap::init(
                    unsafe { transmute_ref(&mut container) },
                    cap as u64 * 64,
                    base,
                )
            };
            Self { bitmap, container }
        }

        pub fn bitmap(&'a mut self) -> &mut Bitmap<'_> {
            &mut self.bitmap
        }
    }
}
