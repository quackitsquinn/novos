//! Kernel ELF Parser
//!
//! This crate provides a simple ELF parser for the kernel, allowing you to read sections,
//! symbols, strings, and segments from an ELF file.

#![no_std]

use goblin::{
    elf::section_header::SHT_SYMTAB,
    elf64::{
        header,
        program_header::ProgramHeader,
        reloc::SIZEOF_RELA,
        section_header,
        sym::{SIZEOF_SYM, Sym},
    },
};

pub use goblin;

use crate::{
    phdr::ElfSegments,
    reloc::{ElfRelocation, ElfRelocations},
    sections::ElfSections,
    strings::ElfStrings,
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

    /// Returns the symbols in the ELF file as a slice.
    pub fn symbols_slice(&'a self) -> Option<&'a [Sym]> {
        let symtab = self
            .sections()
            .find(|section| section.sh_type == SHT_SYMTAB)?;

        let size = symtab.sh_size as usize / SIZEOF_SYM;
        let offset = symtab.sh_offset as usize;

        Some(unsafe {
            core::slice::from_raw_parts(self.data.as_ptr().add(offset).cast::<Sym>(), size)
        })
    }

    /// Iterate over the segments in the ELF file.
    pub fn segments(&'a self) -> ElfSegments<'a> {
        unsafe { ElfSegments::new(self.data, self.header) }
    }

    /// Returns the segments in the ELF file as a slice.
    pub fn segments_slice(&'a self) -> Option<&'a [ProgramHeader]> {
        if self.header.e_phnum == 0 {
            return None;
        }
        let size = self.header.e_phnum as usize;
        let offset = self.header.e_phoff as usize;

        Some(unsafe {
            core::slice::from_raw_parts(
                self.data.as_ptr().add(offset).cast::<ProgramHeader>(),
                size,
            )
        })
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

    /// Returns the relocations in the ELF file as a slice.
    pub fn relocations_slice(&'a self) -> Option<&'a [ElfRelocation]> {
        let reloc_section = self
            .sections()
            .find(|section| section.sh_type == section_header::SHT_RELA)?;

        let size = reloc_section.sh_size as usize / SIZEOF_RELA;
        let offset = reloc_section.sh_offset as usize;

        Some(unsafe {
            core::slice::from_raw_parts(
                self.data.as_ptr().add(offset).cast::<ElfRelocation>(),
                size,
            )
        })
    }
}
