use core::mem;

use goblin::elf64::{
    reloc::{self, Rela, SIZEOF_RELA},
    section_header,
};

pub struct ElfRelocations<'a> {
    data: &'a [u8],
    section: &'a section_header::SectionHeader,
    i: usize,
}

impl<'a> ElfRelocations<'a> {
    pub fn new(data: &'a [u8], section: &'a section_header::SectionHeader) -> Self {
        Self {
            data,
            section,
            i: 0,
        }
    }
}

impl<'a> Iterator for ElfRelocations<'a> {
    type Item = ElfRelocation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.section.sh_size as usize / reloc::SIZEOF_RELA {
            let offset = self.section.sh_offset as usize + self.i * reloc::SIZEOF_RELA;
            let item = unsafe { &*(self.data.as_ptr().add(offset).cast::<reloc::Rela>()) };
            self.i += 1;
            Some(ElfRelocation::from_rela(item))
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfRelocation {
    pub offset: u64,
    pub info: RelocationInfo,
    pub addend: i64,
}

const _: () = assert!(mem::size_of::<ElfRelocation>() == SIZEOF_RELA);

impl ElfRelocation {
    pub fn from_rela(rela: &Rela) -> Self {
        // SAFETY: The layout of `Rela` matches that of `ElfRelocation`.
        // The fields are in the same order and have the same types.
        unsafe { mem::transmute_copy(rela) }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelocationInfo(u64);

impl RelocationInfo {
    pub fn index(&self) -> u64 {
        self.0 >> 32
    }

    pub fn reloc_type(&self) -> u64 {
        self.0 & 0xFFFFFFFF
    }
}
