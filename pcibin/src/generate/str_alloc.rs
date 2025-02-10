use std::{rc::Rc, sync::Mutex};

/// A string allocator for binary formats.
pub struct BinaryStringAllocator {
    buffer: Vec<u8>,
    offset: Rc<Mutex<u32>>,
}

impl BinaryStringAllocator {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offset: Rc::new(Mutex::new(0)),
        }
    }

    /// Push a string to the buffer and return a pointer to it.
    /// The string is null-terminated.
    pub fn push_data(&mut self, s: &str) -> StringPtr {
        let start = self.buffer.len();
        self.buffer.extend_from_slice(s.as_bytes());
        self.buffer.push(0);
        let ptr = start as u32;
        StringPtr::new(ptr, self.offset.clone())
    }
    /// Set the offset for the string allocator.
    pub fn set_offset(&self, offset: u32) {
        *self.offset.lock().unwrap() = offset;
    }
}

/// A pointer to a string in the binary format.
/// The pointer is relative to the offset of the string allocator, which can be set with `set_offset` in `BinaryStringAllocator`.
pub struct StringPtr {
    ptr: u32,
    offset: Rc<Mutex<u32>>,
}

impl StringPtr {
    fn new(ptr: u32, offset: Rc<Mutex<u32>>) -> Self {
        Self { ptr, offset }
    }

    pub fn get(&self) -> u32 {
        self.ptr + *self.offset.lock().unwrap()
    }
}
