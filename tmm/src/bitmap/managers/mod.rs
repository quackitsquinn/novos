use core::mem::Alignment;

use crate::{arch::L1_PAGE_SIZE, bitmap::BitPtr, paging::Address};

pub mod phys;
pub mod virt;

/// The number of u64 entries required to represent a given number of bytes in a bitmap.
const fn entries_for_bytes(bytes: u64) -> u64 {
    n_pages_for_bytes(bytes).div_ceil(u64::BITS as u64)
}

/// The bit alignment required to represent a given byte alignment in a bitmap.
const fn align_in_bits(byte_alignment: Alignment) -> Alignment {
    Alignment::new(byte_alignment.as_usize().div_ceil(L1_PAGE_SIZE as usize)).unwrap()
}

/// The number of pages required to represent a given number of bytes, based on the L1 page size.
const fn n_pages_for_bytes(bytes: u64) -> u64 {
    bytes.div_ceil(L1_PAGE_SIZE)
}

/// Converts a bit index in a bitmap to the corresponding address, given a base address.
const fn bit_index_as_address<A: const Address>(bit_index: u64, base: A) -> A {
    let offset = bit_index * L1_PAGE_SIZE;
    base + offset
}

/// Converts an address to the corresponding bit index in a bitmap, given a base address.
const fn address_as_bit_index<A: const Address>(address: A, base: A) -> Option<BitPtr> {
    let offset = match address.as_u64().checked_sub(base.as_u64()) {
        Some(offset) => offset,
        None => return None,
    };
    Some(BitPtr::new_wrapping(0, offset / L1_PAGE_SIZE))
}

/// Returns the alignment of the given address as an `Alignment` type.
const fn alignment_of<A: const Address>(address: A) -> Alignment {
    let addr = address.as_u64();
    let alignment = addr & (!addr + 1);

    Alignment::new(alignment as usize).unwrap()
}

#[cfg(test)]
mod tests {
    use core::mem::Alignment;

    use crate::arch::L1_PAGE_SIZE;
    use crate::paging::{Address, VirtAddr};

    #[test]
    fn test_entries_for_bytes() {
        let cases = [
            (0, 0),
            (L1_PAGE_SIZE, 1),
            (L1_PAGE_SIZE * 64, 1),
            (L1_PAGE_SIZE * 128, 2),
            (L1_PAGE_SIZE * 63, 1),
            (L1_PAGE_SIZE * 65, 2),
        ];

        for case in cases {
            let (input, expected) = case;
            let result = super::entries_for_bytes(input);
            assert_eq!(
                result, expected,
                "entries_for_bytes({}) returned {}, expected {}",
                input, result, expected
            );
        }
    }

    #[test]
    fn test_align_in_bits() {
        let cases = [
            (1, 1),
            (L1_PAGE_SIZE, 1),
            (L1_PAGE_SIZE * 64, 64),
            (L1_PAGE_SIZE * 128, 128),
        ];

        for case in cases {
            let (input, expected) = case;
            let alignment = Alignment::new(input as usize).unwrap();
            let result = super::align_in_bits(alignment);
            assert_eq!(
                result.as_usize(),
                expected,
                "align_in_bits({}) returned {}, expected {}",
                input,
                result.as_usize(),
                expected
            );
        }
    }

    #[test]
    fn test_n_pages_for_bytes() {
        let cases = [
            (0, 0),
            (L1_PAGE_SIZE, 1),
            (L1_PAGE_SIZE * 64, 64),
            (L1_PAGE_SIZE * 128, 128),
        ];

        for case in cases {
            let (input, expected) = case;
            let result = super::n_pages_for_bytes(input);
            assert_eq!(
                result, expected,
                "n_pages_for_bytes({}) returned {}, expected {}",
                input, result, expected
            );
        }
    }

    #[test]
    fn test_bit_index_as_address() {
        let base = VirtAddr::new(0x1000_0000);
        let cases = [
            (0, base),
            (1, base + L1_PAGE_SIZE),
            (64, base + L1_PAGE_SIZE * 64),
            (72, base + L1_PAGE_SIZE * 72),
            (128, base + L1_PAGE_SIZE * 128),
        ];

        for case in cases {
            let (bit_index, expected) = case;
            let result = super::bit_index_as_address(bit_index, base);
            assert_eq!(
                result,
                expected,
                "bit_index_as_address({}, {:#x}) returned {:#x}, expected {:#x}",
                bit_index,
                base.as_u64(),
                result.as_u64(),
                expected.as_u64()
            );
        }
    }

    #[test]
    fn test_address_as_bit_index() {
        let base = VirtAddr::new(0x1000_0000);
        let cases = [
            (base, Some(0)),
            (base + L1_PAGE_SIZE, Some(1)),
            (base + L1_PAGE_SIZE * 64, Some(64)),
            (base + L1_PAGE_SIZE * 72, Some(72)),
            (base + L1_PAGE_SIZE * 128, Some(128)),
            (base - 1, None),
        ];

        for case in cases {
            let (address, expected) = case;
            let result = super::address_as_bit_index(address, base);
            let expected = expected.map(|bit_index| super::BitPtr::new_wrapping(0, bit_index));
            assert_eq!(
                result,
                expected,
                "address_as_bit_index({:#x}, {:#x}) returned {:?}, expected {:?}",
                address.as_u64(),
                base.as_u64(),
                result,
                expected
            );
        }
    }

    #[test]
    fn test_alignment_of() {
        let cases = [
            (0x1000_0000, 0x1000_0000),
            (0x1000_0001, 1),
            (0x1000_0002, 2),
            (0x1000_0003, 1),
            (0x1000_0004, 4),
            (0x1000_0005, 1),
            (0x1000_0006, 2),
            (0x1000_0007, 1),
            (0x1000_0008, 8),
        ];

        for case in cases {
            let (address, expected) = case;
            let addr = VirtAddr::new(address);
            let result = super::alignment_of(addr);
            assert_eq!(
                result.as_usize(),
                expected,
                "alignment_of({:#x}) returned {}, expected {}",
                address,
                result.as_usize(),
                expected
            );
        }
    }
}
