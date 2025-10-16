//! Post page table switch data structures and abstractions.
use core::ops::Deref;

use arrayvec::ArrayVec;
use cake::{
    LimineData,
    limine::{memory_map::Entry, response::MemoryMapResponse},
};

// FIXME: Somehow account for Entry not implementing Debug for totally sane reasons.

/// Represents the system's memory map.
#[allow(missing_debug_implementations)]
pub struct MemoryMap {
    entries: ArrayVec<Entry, 256>,
}

impl MemoryMap {
    /// Creates a new MemoryMap from a Limine MemoryMapResponse.
    pub fn new(response: LimineData<'_, MemoryMapResponse>) -> Self {
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
