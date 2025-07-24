use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
    ptr::addr_of,
};

use cake::Owned;
use x86_64::{
    VirtAddr,
    structures::paging::{Page, PageTable},
};

use crate::{
    KernelPage,
    pagetable::{
        PageTablePath,
        entry::{Entry, PageIndex},
    },
};

pub struct PageLayout {
    entries: [MaybeUninit<Entry>; 255],
    len: u32,
    _unused: u32,
    next: Option<Owned<PageLayout>>,
}

const _: () = assert!(
    size_of::<PageLayout>() == 4096,
    "PageLayout must be exactly 4096 bytes"
);

impl PageLayout {
    pub unsafe fn create_in_page(page: KernelPage) -> Owned<Self> {
        let ptr = page.start_address().as_mut_ptr::<PageLayout>();
        unsafe {
            ptr.write_bytes(0, 1);
            Owned::new(ptr)
        }
    }

    pub fn push(&mut self, pagetable: Owned<PageTable>, path: PageTablePath) -> &mut PageTable {
        self.try_push(pagetable, path).expect("push failed")
    }

    pub fn try_push(
        &mut self,
        pagetable: Owned<PageTable>,
        path: PageTablePath,
    ) -> Result<&mut PageTable, &'static str> {
        if self.len >= self.entries.len() as u32 {
            if let Some(next) = &mut self.next {
                return next.try_push(pagetable, path);
            } else {
                return Err("no space");
            }
        }
        let entry = Entry::new(pagetable, path);

        self.entries[self.len as usize].write(entry);

        self.len += 1;
        Ok(unsafe {
            self.entries[self.len as usize - 1]
                .assume_init_mut()
                .pagetable_mut()
        })
    }

    pub fn has_cap(&self, needed: usize) -> bool {
        let remaining = self.entries.len() - self.len as usize;
        if needed > remaining {
            if let Some(next) = &self.next {
                return next.has_cap(needed - remaining);
            }
            return false;
        }
        return true;
    }

    pub fn extend(&mut self, new: KernelPage) {
        if let Some(next) = &mut self.next {
            next.extend(new);
        } else {
            let next = unsafe { PageLayout::create_in_page(new) };
            self.next = Some(next);
        }
    }

    pub fn iter(&self) -> PageLayoutIter {
        PageLayoutIter {
            layout: self,
            index: 0,
        }
    }

    pub fn get(&self, indexes: PageTablePath) -> Option<&Entry> {
        self.iter().find(|entry| entry.path() == indexes)
    }

    pub fn get_mut(&mut self, indexes: PageTablePath) -> Option<&mut Entry> {
        for entry in self.entries[..self.len as usize].iter_mut() {
            let entry = unsafe { &mut *entry.as_mut_ptr() };
            if entry.path() == indexes {
                return Some(entry);
            }
        }

        if let Some(next) = &mut self.next {
            return next.get_mut(indexes);
        }

        None
    }

    pub fn index_of(&self, indexes: PageTablePath) -> Option<usize> {
        let packed = PageIndex::pack(indexes);
        for (i, entry) in self.iter().enumerate() {
            if entry.raw_path() == packed {
                return Some(i);
            }
        }
        None
    }

    pub fn contains(&self, indexes: PageTablePath) -> bool {
        self.index_of(indexes).is_some()
    }

    /// Reclaims the pages in the layout, calling `page_dealloc` for each page.
    /// This will *not* deallocate any entries in the layout, only the layout itself.
    pub unsafe fn reclaim(&mut self, page_dealloc: unsafe fn(KernelPage)) {
        let mut cur = self.next.take();

        while let Some(mut next) = cur.take() {
            let next_page = next.next.take();
            let page = VirtAddr::from_ptr(next.into_raw().cast_const());
            unsafe { page_dealloc(KernelPage::from_start_address(page).expect("unaligned")) };
            cur = next_page;
        }
    }
}

impl Index<usize> for PageLayout {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        if index < self.len as usize {
            return unsafe { &*self.entries[index].as_ptr() };
        }

        if let Some(next) = &self.next {
            return &next[index - self.len as usize];
        }

        panic!(
            "Index out of bounds: {} for PageLayout with length {}",
            index, self.len
        );
    }
}

impl IndexMut<usize> for PageLayout {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.len as usize {
            return unsafe { &mut *self.entries[index].as_mut_ptr() };
        }

        if let Some(next) = &mut self.next {
            return &mut next[index - self.len as usize];
        }

        panic!(
            "Index out of bounds: {} for PageLayout with length {}",
            index, self.len
        );
    }
}

pub struct PageLayoutIter<'a> {
    layout: &'a PageLayout,
    index: u32,
}

impl<'a> Iterator for PageLayoutIter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.layout.len {
            let entry = &self.layout[self.index as usize];
            self.index += 1;
            return Some(entry);
        }

        if let Some(next) = &self.layout.next {
            self.layout = next;
            self.index = 0;
            return self.next();
        }

        None
    }
}
