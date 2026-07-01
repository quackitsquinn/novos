use core::{
    alloc::Layout,
    mem::{Alignment, MaybeUninit},
};

use cake::limine::memory_map::EntryType;

use crate::{
    MapFlags, MemError, align,
    bitmap::{Bitmap, VirtualMemoryManager},
    entry_walker::EntryWalker,
    paging::{
        Address, AddressExt, FragmentSize, PhysAddr, Small, VirtAddr, map_from,
        primitives::MemoryRange,
    },
};

pub struct PhysicalMemoryManager {
    bitmaps: &'static mut [BitmapEntry],
}

impl PhysicalMemoryManager {
    pub unsafe fn init(
        entry_walker: &mut EntryWalker,
        vmm: &mut VirtualMemoryManager,
    ) -> Result<Self, MemError> {
        let entries = entry_walker.entries;
        let n_avail = entries
            .iter()
            .filter(|e| e.entry_type == EntryType::USABLE)
            .count();

        let slice_layout = Layout::array::<BitmapEntry>(n_avail).unwrap();
        let vmem = vmm
            .allocate(slice_layout)
            .expect("Failed to allocate memory for bitmap entries");

        unsafe {
            map_from(
                vmem,
                slice_layout.size() as u64,
                MapFlags::WRITABLE,
                entry_walker,
            )?
        };

        let bitmaps = unsafe {
            core::slice::from_raw_parts_mut(vmem.as_mut_ptr::<MaybeUninit<BitmapEntry>>(), n_avail)
        };

        let mut bitmap_iter = entries.iter().filter(|e| e.entry_type == EntryType::USABLE);
        for i in 0..n_avail {
            let entry = bitmap_iter.next().unwrap();
            bitmaps[i] = MaybeUninit::new(Self::allocate_bitmap(
                MemoryRange::new_len(PhysAddr::new(entry.base), entry.length),
                entry_walker,
                vmm,
            )?);
        }

        Ok(Self {
            bitmaps: unsafe { core::mem::transmute(bitmaps) },
        })
    }

    fn allocate_bitmap(
        range: MemoryRange<PhysAddr>,
        walker: &mut EntryWalker,
        vmm: &mut VirtualMemoryManager,
    ) -> Result<BitmapEntry, crate::MemError> {
        let needed_bytes = align!(
            up,
            Self::bytes_for_size(range.size()),
            core::mem::size_of::<u64>() as u64
        );
        let needed_entries = needed_bytes.div_ceil(core::mem::size_of::<u64>() as u64);
        let virtual_start = vmm
            .allocate(Layout::from_size_align(needed_bytes as usize, 8).unwrap())
            .ok_or(MemError::OutOfMemory)?;
        unsafe { map_from(virtual_start, needed_bytes, MapFlags::WRITABLE, walker)? };

        let bitmap_slice = unsafe {
            core::slice::from_raw_parts_mut(
                virtual_start.as_mut_ptr::<u64>(),
                needed_entries as usize,
            )
        };

        Ok(BitmapEntry {
            bitmap: Bitmap::init(
                bitmap_slice,
                range.size() / Small::SIZE,
                range.start().as_u64(),
            ),
            alignment: Self::alignment_for(range.start()),
            free: range.size() / Small::SIZE,
        })
    }

    const fn bits_for_size(size_bytes: u64) -> u64 {
        size_bytes.div_ceil(Small::SIZE)
    }

    const fn bytes_for_size(size_bytes: u64) -> u64 {
        let bits = Self::bits_for_size(size_bytes);
        bits.div_ceil(8)
    }

    const fn alignment_for(addr: impl const Address) -> Alignment {
        let addr = addr.as_u64();
        if addr == 0 {
            return Alignment::new(1).unwrap();
        }

        Alignment::new(1 << (addr.trailing_zeros())).unwrap()
    }
}

struct BitmapEntry {
    // contains the base address of the range
    bitmap: Bitmap<'static>,
    /// the max alignment that this bitmap can guarantee for it's allocations.
    ///
    /// there will be a way to configure how many entries in the manager that are aligned to higher alignments,
    /// which removes any alignment logic from the bitmap itself. ideally there will be ranges that do map well to higher alignments, but
    /// we can just align up to the next alignment boundary, which yes, does waste some memory, but it is simpler and removes the weird alignment logic that a unaligned
    /// bitmap would require.
    alignment: Alignment,
    /// the total number of free frames in this bitmap. used to skip bitmaps that are full.
    free: u64,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_bits_for_size() {
        assert_eq!(PhysicalMemoryManager::bits_for_size(4096), 1);
        assert_eq!(PhysicalMemoryManager::bits_for_size(8192), 2);
        assert_eq!(PhysicalMemoryManager::bits_for_size(4095), 1);
        assert_eq!(PhysicalMemoryManager::bits_for_size(4097), 2);
        assert_eq!(PhysicalMemoryManager::bits_for_size(0), 0);
    }

    #[test]
    fn test_bytes_for_size() {
        assert_eq!(PhysicalMemoryManager::bytes_for_size(4096), 1);
        assert_eq!(PhysicalMemoryManager::bytes_for_size(8192), 1);
        assert_eq!(PhysicalMemoryManager::bytes_for_size(4095), 1);
        assert_eq!(PhysicalMemoryManager::bytes_for_size(4097), 1);
        assert_eq!(PhysicalMemoryManager::bytes_for_size(0), 0);
    }

    #[test]
    fn test_alignment_for() {
        #[rustfmt::skip]
        let cases = [(0, 1), (1,  1), (2,  2), (3,  1), (4,  4), (5,  1), (6,  2), (7,   1), (8,  8),
                                       (9, 1), (10, 2), (11, 1), (12, 4), (13, 1), (14, 2), (15, 1), (16, 16), (17, 1)];
        for (addr, expected) in cases {
            let addr = VirtAddr::new(addr);
            let alignment = PhysicalMemoryManager::alignment_for(addr);
            assert_eq!(
                alignment,
                Alignment::new(expected).unwrap(),
                "for address {:#x}",
                addr.as_u64()
            );
        }
    }
}
