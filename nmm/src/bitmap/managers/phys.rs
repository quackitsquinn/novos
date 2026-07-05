use core::{
    alloc::Layout,
    mem::{Alignment, MaybeUninit},
};

use cake::{limine::memory_map::EntryType, log::info};

use crate::{
    MapFlags, MemError, align,
    bitmap::{
        Bitmap, VirtualMemoryManager,
        managers::{align_in_bits, alignment_of, entries_for_bytes},
    },
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

        let bitmaps = unsafe { core::mem::transmute::<_, &mut [BitmapEntry]>(bitmaps) };

        bitmaps.sort_unstable_by_key(|e| e.free);

        for bitmap in bitmaps.iter() {
            info!(
                "Initialized bitmap for range: start={:#x}, size={} bytes, alignment={:x?}, free frames={}",
                bitmap.start.as_u64(),
                bitmap.size(),
                bitmap.byte_alignment(),
                bitmap.free
            );
        }

        Ok(Self { bitmaps })
    }

    fn allocate_bitmap(
        range: MemoryRange<PhysAddr>,
        walker: &mut EntryWalker,
        vmm: &mut VirtualMemoryManager,
    ) -> Result<BitmapEntry, crate::MemError> {
        let needed_entries = entries_for_bytes(range.size());
        let needed_bytes = needed_entries * core::mem::size_of::<u64>() as u64;
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
            start: range.start(),
            bitmap: Bitmap::init(bitmap_slice, range.size() / Small::SIZE),
            bit_alignment: align_in_bits(alignment_of(range.start())),
            free: range.size() / Small::SIZE,
        })
    }
}

struct BitmapEntry {
    // the bitmap that tracks the allocation of frames in this range
    bitmap: Bitmap<'static>,
    // the start of this entry
    start: PhysAddr,
    /// the max alignment that this bitmap can guarantee for it's allocations.
    ///
    /// there will be a way to configure how many entries in the manager that are aligned to higher alignments,
    /// which removes any alignment logic from the bitmap itself. ideally there will be ranges that do map well to higher alignments, but
    /// we can just align up to the next alignment boundary, which yes, does waste some memory, but it is simpler and removes the weird alignment logic that a unaligned
    /// bitmap would require.
    bit_alignment: Alignment,
    /// the total number of free frames in this bitmap. used to skip bitmaps that are full.
    free: u64,
}

impl BitmapEntry {
    fn size(&self) -> u64 {
        self.bitmap.n_bits() * Small::SIZE
    }

    fn byte_alignment(&self) -> Alignment {
        Alignment::new(self.bit_alignment.as_usize() * Small::SIZE as usize).unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
}
