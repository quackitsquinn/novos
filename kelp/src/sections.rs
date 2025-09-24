use goblin::elf64::{header, section_header};

pub struct ElfSections<'a> {
    data: &'a [u8],
    header: &'a header::Header,
    i: usize,
}

impl<'a> ElfSections<'a> {
    /// Create an iterator over the sections in the ELF file.
    /// # Safety
    /// The caller must ensure that the data is a valid ELF file and that the section headers are valid.
    pub unsafe fn new(data: &'a [u8], header: &'a header::Header) -> Self {
        Self { data, header, i: 0 }
    }
}

impl<'a> Iterator for ElfSections<'a> {
    type Item = &'a section_header::SectionHeader;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.header.e_shnum as usize {
            // TODO: Do a little more research on if the size of the section header table is always the same.
            // If so, use a slice instead of pointer arithmetic
            let item = unsafe {
                &*self
                    .data
                    .as_ptr()
                    // Add the offset of the section header table
                    .add(self.header.e_shoff as usize)
                    // Index to the current section header
                    .add(self.i * self.header.e_shentsize as usize)
                    .cast()
            };
            self.i += 1;
            Some(item)
        } else {
            None
        }
    }
}
