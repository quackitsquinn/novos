use goblin::elf64::{header, program_header::ProgramHeader};

/// An iterator over the segments in an ELF file.
pub struct ElfSegments<'a> {
    header: &'a header::Header,
    phdr: *const ProgramHeader,
    i: usize,
}

impl<'a> ElfSegments<'a> {
    pub unsafe fn new(data: &'a [u8], header: &'a header::Header) -> ElfSegments<'a> {
        let phdr = unsafe {
            data.as_ptr()
                .add(header.e_phoff as usize)
                .cast::<ProgramHeader>()
        };
        ElfSegments { header, phdr, i: 0 }
    }
}

impl<'a> Iterator for ElfSegments<'a> {
    type Item = &'a ProgramHeader;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.header.e_phnum as usize {
            // SAFETY: The pointer is valid because we are iterating over the program headers
            let item = unsafe { &*self.phdr.add(self.i) };
            self.i += 1;
            Some(item)
        } else {
            None
        }
    }
}
