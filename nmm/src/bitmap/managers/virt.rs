use core::{alloc::Layout, mem::Alignment};

use crate::{
    arch,
    bitmap::{BitPtr, Bitmap},
    paging::{Address, AddressExt, VirtAddr},
    test_println,
};

/// A memory manager for virtual addresses, which internally is based on a bitmap to track allocated and free virtual address space.
#[derive(Debug)]
pub struct VirtualAddressManager<'a> {
    bitmap: Bitmap<'a>,
}

impl<'a> VirtualAddressManager<'a> {
    const BIT_SIZE: u64 = arch::L1_PAGE_SIZE;

    /// Initializes the `VirtualAddressManager` with the given bitmap data and base address. The bitmap data is a mutable slice of `u64` values that will be used to track allocated and free virtual address space, and the base address is the starting virtual address that the bitmap will manage.
    ///
    /// # Safety
    /// The caller must ensure that the provided bitmap data is valid and that the base address is unused and properly aligned for the size of the bitmap.
    /// The size of the bitmap in bits is determined by the `size` parameter, which specifies the total size of the virtual address space to manage in bytes.
    ///
    ///
    pub unsafe fn init(bitmap_data: &'a mut [u64], base: VirtAddr, size: u64) -> Self {
        Self {
            bitmap: Bitmap::init(bitmap_data, Self::bytes_to_bits(size), base.as_u64()),
        }
    }

    const fn bytes_to_bits(n_bytes: u64) -> u64 {
        (n_bytes + Self::BIT_SIZE - 1) / Self::BIT_SIZE
    }

    const fn align_to_bit_align(align: Alignment) -> Alignment {
        let align = align.as_usize() as u64;
        if align <= Self::BIT_SIZE {
            return Alignment::new(1).unwrap();
        }

        let bit_align = align / Self::BIT_SIZE;
        return Alignment::new(bit_align as usize).unwrap();
    }

    fn bitptr_to_virtaddr(base: u64, bitptr: BitPtr) -> VirtAddr {
        let addr: *const u8 = bitptr.as_ptr(base as *mut _, Self::BIT_SIZE);
        VirtAddr::from_ptr(addr).expect("invalid virtaddr")
    }

    const fn virtaddr_to_bitptr(base: u64, addr: VirtAddr) -> Option<BitPtr> {
        let addr_u64 = addr.as_u64();
        if addr_u64 < base {
            return None;
        }
        let offset = addr_u64 - base;
        if offset % Self::BIT_SIZE != 0 {
            return None;
        }
        let bit_offset = offset / Self::BIT_SIZE;
        Some(BitPtr::new_wrapping(0, bit_offset))
    }

    /// Returns the number of bitmap entries needed to manage a virtual address space of the given size in bytes.
    pub const fn entries_to_fit(bytes: u64) -> u64 {
        let bits = Self::bytes_to_bits(bytes);
        let entries = bits.div_ceil(64);
        entries
    }

    /// Allocates a range of virtual memory of the specified size and alignment, returning the starting virtual address of the allocated range.
    ///
    /// Any allocation has a footprint of at least `crate::arch::L1_PAGE_SIZE` bytes, as the bitmap tracks virtual address space in units of pages.
    /// The `n_bytes` parameter specifies the total size of the virtual address range to allocate in bytes, and the `align` parameter specifies the required alignment of the starting virtual address.
    #[must_use = "the allocated virtual address must be used or deallocated to avoid memory leaks"]
    pub fn allocate(&mut self, layout: Layout) -> Option<VirtAddr> {
        let n_bits = Self::bytes_to_bits(layout.size() as u64);
        let bit_align = Self::align_to_bit_align(layout.alignment());

        let bitptr = self.bitmap.allocate(n_bits, bit_align)?;
        Some(Self::bitptr_to_virtaddr(self.bitmap.base_addr, bitptr))
    }

    /// Deallocates a previously allocated range of virtual memory starting at the given virtual address and spanning the specified number of bytes.
    ///
    pub unsafe fn deallocate(&mut self, addr: VirtAddr, layout: Layout) {
        let n_bits = Self::bytes_to_bits(layout.size() as u64);
        let bitptr = Self::virtaddr_to_bitptr(self.bitmap.base_addr, addr)
            .expect("deallocated address must be within the managed virtual address space and properly aligned");
        test_println!("deallocating addr {:?}, bitptr: {:?}", addr, bitptr);
        debug_assert!(self.bitmap.some_are_set(bitptr, n_bits));
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
    fn test_bytes_to_bits() {
        fn test(bytes: u64, expected_bits: u64) {
            assert_eq!(
                VirtualAddressManager::bytes_to_bits(bytes),
                expected_bits,
                "bytes_to_bits({}) should be {}",
                bytes,
                expected_bits
            );
        }
        test(0, 0);
        test(1, 1);
        test(arch::L1_PAGE_SIZE, 1);
        test(arch::L1_PAGE_SIZE + 1, 2);
        test(arch::L1_PAGE_SIZE * 10, 10);
    }

    #[test]
    fn test_align_to_bit_align() {
        fn test(align: u64, expected_bit_align: u64) {
            let align = Alignment::new(align as usize).expect("given align must be a power of two");
            let expected_bit_align = Alignment::new(expected_bit_align as usize)
                .expect("given expected_bit_align must be a power of two");
            assert_eq!(
                VirtualAddressManager::align_to_bit_align(align),
                expected_bit_align,
                "align_to_bit_align({:?}) should be {:?}",
                align,
                expected_bit_align
            );
        }

        test(1, 1);
        test(arch::L1_PAGE_SIZE / 2, 1);
        test(arch::L1_PAGE_SIZE, 1);
        test(arch::L1_PAGE_SIZE * 2, 2);
    }

    #[test]
    fn test_bitptr_to_virtaddr() {
        fn test(base: u64, bit_offset: u64, expected_addr: u64) {
            let bitptr = BitPtr::new_wrapping(0, bit_offset);
            assert_eq!(
                VirtualAddressManager::bitptr_to_virtaddr(base, bitptr).as_u64(),
                expected_addr,
                "bitptr_to_virtaddr({}, {:?}) should be {}",
                base,
                bitptr,
                expected_addr
            );
        }

        test(0x1000, 0, 0x1000);
        test(0x1000, 1, 0x1000 + arch::L1_PAGE_SIZE);
        test(0x1000, 2, 0x1000 + 2 * arch::L1_PAGE_SIZE);
        for i in 0..0x10_000 {
            test(0x1000, i, 0x1000 + i * arch::L1_PAGE_SIZE);
        }
    }

    #[test]
    fn test_virtaddr_to_bitptr() {
        fn test(base: u64, addr: u64, expected_bit_offset: Option<u64>) {
            let addr = VirtAddr::new(addr);
            let expected_bitptr = expected_bit_offset.map(|offset| BitPtr::new_wrapping(0, offset));
            assert_eq!(
                VirtualAddressManager::virtaddr_to_bitptr(base, addr),
                expected_bitptr,
                "virtaddr_to_bitptr({}, {:?}) should be {:?}",
                base,
                addr,
                expected_bitptr
            );
        }
        test(0x1000, 0x1000, Some(0));
        test(0x1000, 0x1000 + arch::L1_PAGE_SIZE, Some(1));
        test(0x1000, 0x1000 + 2 * arch::L1_PAGE_SIZE, Some(2));
        test(0x1000, 0x1000 + (arch::L1_PAGE_SIZE / 2), None);
        test(0x1000, 0x0FFF, None);
        test(0x1000, 0x10000, Some(15));
    }

    #[test]
    fn test_allocate_and_deallocate() {
        const CAP: u64 = 512;
        let mut bitmap_data = [0u64; CAP as usize];
        let base = VirtAddr::new(0x10000000);
        let base_u64 = base.as_u64();
        let mut manager =
            unsafe { VirtualAddressManager::init(&mut bitmap_data, base, 0x1000 * (64 * CAP)) };

        let test = |manager: &mut VirtualAddressManager,
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
            let bits = VirtualAddressManager::bytes_to_bits(size_bytes);
            let bitptr = VirtualAddressManager::virtaddr_to_bitptr(manager.bitmap.base_addr, addr)
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
            |manager: &mut VirtualAddressManager, addr: VirtAddr, layout: Layout| {
                assert!(
                    manager.bitmap.all_are_set(
                        VirtualAddressManager::virtaddr_to_bitptr(manager.bitmap.base_addr, addr)
                            .unwrap(),
                        1
                    ),
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
