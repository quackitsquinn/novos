use std::{rc::Rc, sync::Mutex};

pub struct BinaryOffsetAllocator {
    buf: Vec<u8>,
    offset: u32,
}

pub struct BinaryPtr {
    ptr: u32,
    offset: Rc<Mutex<u32>>,
}

impl BinaryOffsetAllocator {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            offset: 0,
        }
    }

    pub fn push_data(&mut self, data: &[u8]) -> BinaryPtr {
        let start = self.buf.len();
        self.buf.extend_from_slice(data);
        let ptr = start as u32;
        BinaryPtr::new(ptr, self.offset)
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.offset = offset;
    }
}
