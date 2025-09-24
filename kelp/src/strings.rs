use core::{ffi::CStr, mem::transmute};

use goblin::elf64::section_header;

use crate::Elf;

/// A struct to read strings from an ELF file.
pub struct ElfStrings<'a> {
    data: &'a [u8],
    string_table: &'a section_header::SectionHeader,
}

impl ElfStrings<'_> {
    pub unsafe fn new<'a>(data: &'a [u8], elf: &'a Elf<'a>) -> ElfStrings<'a> {
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

unsafe fn read_nul_terminated_str(data: &[u8], offset: usize) -> Option<&CStr> {
    let mut len = 0;
    while data[offset + len] != 0 {
        if len + offset + 1 >= data.len() {
            return None;
        }
        len += 1;
    }
    // SAFETY: A. The loop above ensures that the string is nul-terminated, and B
    //         B. Even if it wasn't, the safety of CStr::from_bytes_with_nul_unchecked is guaranteed by the caller.
    Some(unsafe { CStr::from_bytes_with_nul_unchecked(&data[offset..offset + len + 1]) })
}
