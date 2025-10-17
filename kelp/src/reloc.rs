use core::mem;

use goblin::elf64::reloc::{Rela, SIZEOF_RELA};

/// An ELF relocation entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfRelocation {
    /// The offset of the relocation.
    pub offset: u64,
    /// The relocation info.
    pub info: RelocationInfo,
    /// The addend for the relocation.
    pub addend: i64,
}

const _: () = assert!(mem::size_of::<ElfRelocation>() == SIZEOF_RELA);

impl ElfRelocation {
    /// Creates an `ElfRelocation` from a goblin `Rela`.
    pub fn from_rela(rela: &Rela) -> Self {
        // SAFETY: The layout of `Rela` matches that of `ElfRelocation`.
        // The fields are in the same order and have the same types.
        unsafe { mem::transmute_copy(rela) }
    }
}

/// The relocation info field in an ELF relocation entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelocationInfo(u64);

impl RelocationInfo {
    /// Returns the symbol index for this relocation.
    pub fn index(&self) -> u64 {
        self.0 >> 32
    }

    /// Returns the type of this relocation.
    pub fn kind(&self) -> u64 {
        self.0 & 0xFFFFFFFF
    }
}
