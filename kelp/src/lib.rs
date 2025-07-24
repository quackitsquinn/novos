//! Kernel ELF Parser
//!
//! This crate provides a simple ELF parser for the kernel, allowing you to read sections,
//! symbols, strings, and segments from an ELF file.

#![no_std]

use goblin::{
    elf::section_header::SHT_SYMTAB,
    elf64::{header, section_header},
};

pub use goblin;

use crate::{
    phdr::ElfSegments, reloc::ElfRelocations, sections::ElfSections, strings::ElfStrings,
    sym::ElfSymbols,
};

pub mod phdr;
pub mod reloc;
pub mod sections;
pub mod strings;
pub mod sym;

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
        unsafe { ElfSections::new(self.data, self.header) }
    }

    /// Iterate over the symbols in the ELF file.
    pub fn symbols(&'a self) -> Option<ElfSymbols<'a>> {
        let symtab = self
            .sections()
            .find(|section| section.sh_type == SHT_SYMTAB)?;

        Some(unsafe { ElfSymbols::new(self.data, symtab) })
    }

    /// Iterate over the segments in the ELF file.
    pub fn segments(&'a self) -> ElfSegments<'a> {
        unsafe { ElfSegments::new(self.data, self.header) }
    }

    /// Returns the string table in the ELF file.
    pub fn strings(&'a self) -> Option<ElfStrings<'a>> {
        Some(unsafe { ElfStrings::new(self.data, self) })
    }

    /// Returns the relocations in the ELF file.
    pub fn relocations(&'a self) -> Option<ElfRelocations<'a>> {
        let reloc_section = self
            .sections()
            .find(|section| section.sh_type == section_header::SHT_RELA)?;

        Some(ElfRelocations::new(self.data, reloc_section))
    }
}
