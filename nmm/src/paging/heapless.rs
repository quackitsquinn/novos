//! Early Heap-like structures for use before the heap is initialized.
//!
//! Be warned: every allocation has a footprint of at least `crate::arch::L1_PAGE_SIZE` bytes.

use core::{
    alloc::Layout,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::{self, NonNull},
};

use crate::{
    MapFlags, MapSource, MemError, align, map,
    paging::{Address, AddressExt, Page, Small, VirtAddr, map::GlobalMemoryProvider, map_from},
    reserve_virtual, unmap,
};

/// A Page-backed vector type.
#[derive(Debug, PartialEq, Eq)]
pub struct PageVec<T> {
    base: NonNull<T>,
    len: usize,
    cap: usize,
    flags: Option<MapFlags>,
    _marker: core::marker::PhantomData<T>,
}

impl<T> PageVec<T> {
    /// Creates a new, empty `PageVec` with the specified capacity.
    pub fn new(flags: Option<MapFlags>) -> Self {
        Self {
            base: NonNull::dangling(),
            len: 0,
            cap: 0,
            flags,
            _marker: core::marker::PhantomData,
        }
    }

    fn reallocate(&mut self) -> Result<(), MemError> {
        let (old_layout, _) = layout_for::<T>(self.cap);
        let (layout, cap) = layout_for::<T>(self.cap + 1);
        let range = reserve_virtual(layout)?;
        let flags = self.flags.unwrap_or(MapFlags::empty()) | MapFlags::WRITABLE;
        map(range, MapSource::Anon { zero: false }, layout.size(), flags)?;
        // SAFETY: The range is valid for `layout.size()` bytes, which is enough to hold `cap` elements of type `T`.
        unsafe { ptr::copy_nonoverlapping(self.as_ptr(), range.as_mut_ptr(), self.len) };
        // SAFETY: We construct the layout the exact same way as we did for the allocation, so it is guaranteed to be the same.
        unsafe {
            crate::unmap(
                VirtAddr::from_mut_ptr(self.as_mut_ptr()).unwrap(),
                old_layout.size(),
            )?;
        }

        self.cap = cap;
        self.base =
            NonNull::new(range.as_mut_ptr()).expect("Failed to create NonNull pointer for PageVec");

        Ok(())
    }

    fn as_slice(&self) -> &[T] {
        // SAFETY: If `base` is dangling, then `len` is 0, and this will return an empty slice. Otherwise, `base` is valid for `len` elements.
        unsafe { core::slice::from_raw_parts(self.base.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: If `base` is dangling, then `len` is 0, and this will return an empty slice. Otherwise, `base` is valid for `len` elements.
        unsafe { core::slice::from_raw_parts_mut(self.base.as_ptr(), self.len) }
    }

    /// Pushes a value onto the end of the `PageVec`, reallocating if necessary.
    pub fn push(&mut self, value: T) -> Result<(), MemError> {
        if self.len == self.cap {
            self.reallocate()?;
        }
        // SAFETY: `self.len < self.cap`, so `self.base` is valid for `self.len + 1` elements.
        unsafe { ptr::write(self.base.as_ptr().add(self.len), value) };
        self.len += 1;
        Ok(())
    }

    /// Pops a value off the end of the `PageVec`, returning `None` if the `PageVec` is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // SAFETY: `self.len < self.cap`, so `self.base` is valid for `self.len + 1` elements.
            Some(unsafe { ptr::read(self.base.as_ptr().add(self.len)) })
        }
    }
}

impl<T> Index<usize> for PageVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T> IndexMut<usize> for PageVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<T> Deref for PageVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for PageVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Drop for PageVec<T> {
    fn drop(&mut self) {
        // SAFETY: `self.base` is valid for `self.len` elements.
        unsafe { ptr::drop_in_place(self.as_mut_slice()) };
        if self.cap > 0 {
            let (layout, _) = layout_for::<T>(self.cap);
            unsafe {
                crate::unmap(
                    VirtAddr::from_mut_ptr(self.as_mut_ptr()).unwrap(),
                    layout.size(),
                )
            }
            .expect("Failed to unmap PageVec memory");
        }
    }
}

fn layout_for<T>(len: usize) -> (Layout, usize) {
    let layout = Layout::array::<T>(len).expect("Layout overflow");
    let aligned_size = align!(up, layout.size(), crate::arch::L1_PAGE_SIZE as usize);
    let aligned_align = align!(up, layout.align(), crate::arch::L1_PAGE_SIZE as usize);
    let size = core::mem::size_of::<T>();

    let aligned_layout =
        Layout::from_size_align(aligned_size, aligned_align).expect("Aligned layout overflow");
    let cap = if size != 0 {
        aligned_size / size
    } else {
        usize::MAX
    };
    (aligned_layout, cap)
}
