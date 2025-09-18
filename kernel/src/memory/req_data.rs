use core::ops::Deref;

use arrayvec::ArrayVec;
use cake::limine::{memory_map::Entry, response::MemoryMapResponse};

// FIXME: Somehow account for Entry not implementing Debug for totally sane reasons.
#[allow(missing_debug_implementations)]
pub struct MemoryMap {
    entries: ArrayVec<Entry, 256>,
}

impl MemoryMap {
    pub fn new(response: &MemoryMapResponse) -> Self {
        let mut array = ArrayVec::<Entry, 256>::new();
        for entry in response.entries() {
            array.push(**entry);
        }
        Self { entries: array }
    }
}

impl Deref for MemoryMap {
    type Target = [Entry];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}
