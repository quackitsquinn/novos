use core::{alloc::Layout, mem::Alignment};

use crate::{
    arch,
    bitmap::{
        BitPtr, Bitmap,
        managers::{address_as_bit_index, align_in_bits, bit_index_as_address, n_pages_for_bytes},
    },
    paging::{Address, AddressExt, VirtAddr, primitives::MemoryRange},
    test_println,
};

/// A memory manager for virtual addresses, which internally is based on a bitmap to track allocated and free virtual address space.
#[derive(Debug)]
pub struct VirtualMemoryManager<'a> {
    bitmap: Bitmap<'a>,
    base_addr: VirtAddr,
}

impl<'a> VirtualMemoryManager<'a> {
    const BIT_SIZE: u64 = arch::L1_PAGE_SIZE;

    /// Initializes the `VirtualAddressManager` with the given bitmap data and base address. The bitmap data is a mutable slice of `u64` values that will be used to track allocated and free virtual address space, and the base address is the starting virtual address that the bitmap will manage.
    ///
    /// # Safety
    /// The caller must ensure that the provided bitmap data is valid and that the base address is unused and properly aligned for the size of the bitmap.
    /// The size of the bitmap in bits is determined by the `size` parameter, which specifies the total size of the virtual address space to manage in bytes.
    ///
    ///
    pub unsafe fn init(bitmap_data: &'a mut [u64], range: MemoryRange<VirtAddr>) -> Self {
        Self {
            bitmap: Bitmap::init(bitmap_data, n_pages_for_bytes(range.size())),
            base_addr: range.start(),
        }
    }

    /// Allocates a range of virtual memory of the specified size and alignment, returning the starting virtual address of the allocated range.
    ///
    /// Any allocation has a footprint of at least `crate::arch::L1_PAGE_SIZE` bytes, as the bitmap tracks virtual address space in units of pages.
    /// The `n_bytes` parameter specifies the total size of the virtual address range to allocate in bytes, and the `align` parameter specifies the required alignment of the starting virtual address.
    #[must_use = "the allocated virtual address must be used or deallocated to avoid memory leaks"]
    pub fn allocate(&mut self, layout: Layout) -> Option<VirtAddr> {
        // TODO: Result<VirtAddr, MemError> instead of Option
        let n_bits = n_pages_for_bytes(layout.size() as u64);
        let bit_align = align_in_bits(layout.alignment());

        let bitptr = self.bitmap.allocate(n_bits, bit_align)?;
        Some(bit_index_as_address(bitptr.bit_index(), self.base_addr))
    }

    /// Deallocates a previously allocated range of virtual memory starting at the given virtual address and spanning the specified number of bytes.
    ///
    pub unsafe fn deallocate(&mut self, addr: VirtAddr, layout: Layout) {
        let n_bits = n_pages_for_bytes(layout.size() as u64);
        let bitptr = address_as_bit_index(addr, self.base_addr)
            .expect("deallocated address must be within the managed virtual address space and properly aligned");
        test_println!("deallocating addr {:?}, bitptr: {:?}", addr, bitptr);
        debug_assert!(self.bitmap.some_are_set(bitptr, n_bits));
        self.bitmap.clear(bitptr, n_bits);
    }

    /// Marks a range of virtual memory as allocated in the bitmap, starting at the given virtual address and spanning the specified number of bytes.
    pub unsafe fn mark_allocated(&mut self, addr: VirtAddr, size_bytes: u64) {
        let n_bits = n_pages_for_bytes(size_bytes);
        let bitptr = address_as_bit_index(addr, self.base_addr).expect(
            "address must be within the managed virtual address space and properly aligned",
        );

        self.bitmap.set(bitptr, n_bits);
    }

    /// Marks a range of virtual memory as unallocated in the bitmap, starting at the given virtual address and spanning the specified number of bytes.
    pub unsafe fn mark_unallocated(&mut self, addr: VirtAddr, size_bytes: u64) {
        let n_bits = n_pages_for_bytes(size_bytes);
        let bitptr = address_as_bit_index(addr, self.base_addr).expect(
            "address must be within the managed virtual address space and properly aligned",
        );

        self.bitmap.clear(bitptr, n_bits);
    }

    #[cfg(test)]
    fn dump_entries(&self) {
        const CHUNK_SIZE: usize = 8;
        for (i, entries) in self.bitmap.data.chunks(CHUNK_SIZE).enumerate() {
            print!(
                "entry {:04x}/{:04x}: ",
                i * CHUNK_SIZE,
                (i * CHUNK_SIZE) + (CHUNK_SIZE - 1),
            );
            for entry in entries {
                print!("{:016x}", entry);
            }
            println!();
        }
    }
    #[cfg(test)]
    fn check_zero(&self) {
        for (i, entry) in self.bitmap.data.iter().enumerate() {
            assert_eq!(
                *entry, 0,
                "bitmap entry {} should be zero, but is {:064b}",
                i, entry
            );
        }
    }
    /// Resets the entire bitmap, marking all virtual address space as free.
    /// This is a potentially dangerous operation that should only be used in testing or when the caller is certain that all
    /// previously allocated virtual addresses are no longer in use.
    #[allow(dead_code)] // only used for testing right now
    pub(crate) unsafe fn reset(&mut self) {
        self.bitmap.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_and_deallocate() {
        const CAP: u64 = 512;
        let mut bitmap_data = [0u64; CAP as usize];
        let base = VirtAddr::new(0x10000000);
        let base_u64 = base.as_u64();
        let mut manager = unsafe {
            VirtualMemoryManager::init(
                &mut bitmap_data,
                MemoryRange::new_len(base, 0x1000 * (64 * CAP)),
            )
        };

        let test = |manager: &mut VirtualMemoryManager,
                    size_bytes: u64,
                    align: u64,
                    deallocate: bool|
         -> Option<(VirtAddr, Layout)> {
            let layout = Layout::from_size_align(size_bytes as usize, align as usize).unwrap();
            let addr = manager.allocate(layout).expect("should allocate memory");
            assert_eq!(
                addr.as_u64() % align,
                0,
                "allocated address should be properly aligned"
            );
            let bits = n_pages_for_bytes(size_bytes);
            let bitptr = address_as_bit_index(addr, manager.base_addr)
                .expect("allocated address must be within the managed virtual address space and properly aligned");
            assert!(
                manager.bitmap.all_are_set(bitptr, bits),
                "allocated range should have its bits set in the bitmap"
            );
            if deallocate {
                unsafe { manager.deallocate(addr, layout) };
                manager.check_zero();
                return None;
            }

            Some((addr, layout))
        };

        // This also covers a previous deallocation bug where a bit gets falsely cleared when deallocating another independent allocation.
        let checked_dealloc =
            |manager: &mut VirtualMemoryManager, addr: VirtAddr, layout: Layout| {
                assert!(
                    manager
                        .bitmap
                        .all_are_set(address_as_bit_index(addr, manager.base_addr).unwrap(), 1),
                    "attempted to deallocate an address that is not currently allocated"
                );
                unsafe { manager.deallocate(addr, layout) };
            };

        // Allocate a couple independent pages and then deallocate them.
        let (va_1, layout) =
            test(&mut manager, arch::L1_PAGE_SIZE, arch::L1_PAGE_SIZE, false).unwrap();
        let (va_2, _) = test(&mut manager, arch::L1_PAGE_SIZE, arch::L1_PAGE_SIZE, false).unwrap();

        checked_dealloc(&mut manager, va_1, layout);
        checked_dealloc(&mut manager, va_2, layout);
        manager.check_zero();

        // Allocate a couple independent pages again, deallocate in the opposite order.
        let (va_1, layout) =
            test(&mut manager, arch::L1_PAGE_SIZE, arch::L1_PAGE_SIZE, false).unwrap();
        let (va_2, _) = test(&mut manager, arch::L1_PAGE_SIZE, arch::L1_PAGE_SIZE, false).unwrap();

        checked_dealloc(&mut manager, va_2, layout);
        checked_dealloc(&mut manager, va_1, layout);
        manager.check_zero();

        // allocate 3 pages twice, ensure that the allocations won't overlap and that the deallocation of the first 3 pages won't affect the second 3 pages
        let (va3, layout) = test(
            &mut manager,
            3 * arch::L1_PAGE_SIZE,
            arch::L1_PAGE_SIZE,
            false,
        )
        .unwrap();
        let (va4, _) = test(
            &mut manager,
            3 * arch::L1_PAGE_SIZE,
            arch::L1_PAGE_SIZE,
            false,
        )
        .unwrap();

        checked_dealloc(&mut manager, va3, layout);
        checked_dealloc(&mut manager, va4, layout);
        manager.check_zero();

        test(&mut manager, arch::L2_PAGE_SIZE, arch::L2_PAGE_SIZE, true);
        let (va5, layout) =
            test(&mut manager, arch::L2_PAGE_SIZE, arch::L2_PAGE_SIZE, false).unwrap();
        let (va6, _) = test(&mut manager, arch::L2_PAGE_SIZE, arch::L2_PAGE_SIZE, false).unwrap();

        checked_dealloc(&mut manager, va5, layout);
        checked_dealloc(&mut manager, va6, layout);
        manager.check_zero();

        let (va7, layout) =
            test(&mut manager, arch::L2_PAGE_SIZE, arch::L2_PAGE_SIZE, false).unwrap();
        let (va8, _) = test(&mut manager, arch::L2_PAGE_SIZE, arch::L2_PAGE_SIZE, false).unwrap();

        checked_dealloc(&mut manager, va8, layout);
        checked_dealloc(&mut manager, va7, layout);
        manager.check_zero();
    }
}
