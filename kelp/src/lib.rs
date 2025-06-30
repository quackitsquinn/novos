//! Kernel ELF Parser
//!
//! This crate provides a simple ELF parser for the kernel, allowing you to read sections,
//! symbols, strings, and segments from an ELF file.

#![no_std]

use core::{ffi::CStr, mem::transmute, str};

use goblin::{
    elf::section_header::SHT_SYMTAB,
    elf64::{header, program_header::ProgramHeader, section_header, sym},
};

pub use goblin;

/// An ELF executable
pub struct Elf<'a> {
    pub data: &'a [u8],
    header: &'a header::Header,
}
#[derive(thiserror::Error, Debug)]
pub enum ElfError {
    #[error("Not enough data: {actual} < {expected}")]
    NotEnoughData { actual: usize, expected: usize },
    #[error("Invalid magic: {actual:?} != {expected:?}")]
    InvalidMagic { actual: [u8; 4], expected: [u8; 4] },
    #[error("Invalid architecture: {actual} != {expected}")]
    InvalidArchitecture { actual: u8, expected: u8 },
}

impl<'a> Elf<'a> {
    /// Create a ELF executable from data
    pub fn new(data: &'a [u8]) -> Result<Elf<'a>, ElfError> {
        if data.len() < header::SIZEOF_EHDR {
            Err(ElfError::NotEnoughData {
                actual: data.len(),
                expected: header::SIZEOF_EHDR,
            })
        } else if &data[..header::SELFMAG] != header::ELFMAG {
            let mut ret = [0u8; 4];
            ret.clone_from_slice(&data[..header::SELFMAG]);
            Err(ElfError::InvalidMagic {
                actual: ret,
                expected: *header::ELFMAG,
            })
        } else if data.get(header::EI_CLASS) != Some(&header::ELFCLASS) {
            Err(ElfError::InvalidArchitecture {
                actual: data[header::EI_CLASS],
                expected: header::ELFCLASS,
            })
        } else {
            Ok(Elf {
                data,
                header: unsafe { &*(data.as_ptr().cast()) },
            })
        }
    }

    /// Iterate over the sections in the ELF file.
    pub fn sections(&'a self) -> ElfSections<'a> {
        ElfSections {
            data: self.data,
            header: self.header,
            i: 0,
        }
    }

    /// Iterate over the symbols in the ELF file.
    pub fn symbols(&'a self) -> Option<ElfSymbols<'a>> {
        let symtab = self
            .sections()
            .find(|section| section.sh_type == SHT_SYMTAB)?;

        Some(ElfSymbols::new(self.data, symtab))
    }

    /// Iterate over the segments in the ELF file.
    pub fn segments(&'a self) -> ElfSegments<'a> {
        ElfSegments::new(self.data, self.header)
    }

    /// Returns the string table in the ELF file.
    pub fn strings(&'a self) -> Option<ElfStrings<'a>> {
        Some(ElfStrings::new(self.data, self))
    }
}

pub struct ElfSections<'a> {
    data: &'a [u8],
    header: &'a header::Header,
    i: usize,
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

/// An iterator over the symbols in an ELF file.
pub struct ElfSymbols<'a> {
    data: &'a [u8],
    symbol_table: &'a section_header::SectionHeader,
    i: usize,
    max: usize,
}

impl<'a> ElfSymbols<'a> {
    fn new(data: &'a [u8], symbol_table: &'a section_header::SectionHeader) -> ElfSymbols<'a> {
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

/// A struct to read strings from an ELF file.
pub struct ElfStrings<'a> {
    data: &'a [u8],
    string_table: &'a section_header::SectionHeader,
}

impl ElfStrings<'_> {
    fn new<'a>(data: &'a [u8], elf: &'a Elf<'a>) -> ElfStrings<'a> {
        // Find the longest string table.
        // Given that this is currently only used for backtracing, this is fine.
        // If this is used for something else, we should probably make this more robust.
        let string_table = elf
            .sections()
            .filter(|section| section.sh_type == section_header::SHT_STRTAB)
            .max_by_key(|section| section.sh_size)
            .expect("No string table found");
        ElfStrings { data, string_table }
    }

    /// Get a string from the string table by index.
    /// # Safety
    /// The index must be valid, i.e. it must be less than the size of the string table.
    /// This implementation currently panics if the index is out of bounds, but this is not guaranteed.
    pub unsafe fn get_str<'a>(&self, index: usize) -> Result<&'a str, &'static str> {
        let true_off = self.string_table.sh_offset as usize + index;
        let string =
            unsafe { read_nul_terminated_str(self.data, true_off).ok_or("Unable to read")? };
        // Safety: We are casting a &str to a &'a str, which is safe because the lifetime of data is 'a
        unsafe {
            transmute(str::from_utf8(string.to_bytes()).map_err(|_| "Unable to convert into utf-8"))
        }
    }
}

/// An iterator over the segments in an ELF file.
pub struct ElfSegments<'a> {
    header: &'a header::Header,
    phdr: *const ProgramHeader,
    i: usize,
}

impl<'a> ElfSegments<'a> {
    fn new(data: &'a [u8], header: &'a header::Header) -> ElfSegments<'a> {
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

unsafe fn read_nul_terminated_str(data: &[u8], offset: usize) -> Option<&CStr> {
    let mut len = 0;
    while data[offset + len] != 0 {
        if len == data.len() {
            return None;
        }
        len += 1;
    }
    // SAFETY: A. The loop above ensures that the string is nul-terminated, and B
    //         B. Even if it wasn't, the safety of CStr::from_bytes_with_nul_unchecked is guaranteed by the caller.
    Some(unsafe { CStr::from_bytes_with_nul_unchecked(&data[offset..offset + len + 1]) })
}
