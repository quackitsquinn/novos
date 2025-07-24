use goblin::elf64::{section_header, sym};

/// An iterator over the symbols in an ELF file.
pub struct ElfSymbols<'a> {
    data: &'a [u8],
    symbol_table: &'a section_header::SectionHeader,
    i: usize,
    max: usize,
}

impl<'a> ElfSymbols<'a> {
    pub unsafe fn new(
        data: &'a [u8],
        symbol_table: &'a section_header::SectionHeader,
    ) -> ElfSymbols<'a> {
        ElfSymbols {
            data,
            symbol_table,
            i: 0,
            max: (symbol_table.sh_size as usize) / sym::SIZEOF_SYM,
        }
    }
}

impl<'a> Iterator for ElfSymbols<'a> {
    type Item = &'a sym::Sym;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.max {
            let item = unsafe {
                &*self
                    .data
                    .as_ptr()
                    // Add the offset of the symbol table
                    .add(self.symbol_table.sh_offset as usize)
                    // Index to the current symbol
                    .add(self.i * sym::SIZEOF_SYM)
                    .cast()
            };
            self.i += 1;
            Some(item)
        } else {
            None
        }
    }
}
