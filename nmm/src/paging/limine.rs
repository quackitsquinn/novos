use core::fmt::Debug;

use cake::limine::memory_map::EntryType;

/// This struct follows the same layout as the entries in the Limine memory map response,
/// but is defined here because `limine::memory_map::Entry` does not implement
/// most standard traits like `Debug`, `Clone`, `Copy`, etc, which makes it difficult to work with in the rest of the codebase.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LimineEntry {
    /// The base of the memory region, in *physical space*.
    pub base: u64,
    /// The length of the memory region, in bytes.
    pub length: u64,
    /// The type of the memory region. See [`EntryType`] for specific values.
    pub entry_type: EntryType,
}

impl Debug for LimineEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LimineEntry")
            .field("base", &format_args!("{:#X}", self.base))
            .field("length", &format_args!("{:#X}", self.length))
            .field("entry_type", &fmt_entry_type(self.entry_type))
            .finish()
    }
}

fn fmt_entry_type(entry_type: EntryType) -> &'static str {
    match entry_type {
        EntryType::USABLE => "usable",
        EntryType::RESERVED => "reserved",
        EntryType::ACPI_RECLAIMABLE => "acpi_reclaimable",
        EntryType::ACPI_NVS => "acpi_nvs",
        EntryType::BAD_MEMORY => "bad_memory",
        EntryType::BOOTLOADER_RECLAIMABLE => "bootloader_reclaimable",
        EntryType::EXECUTABLE_AND_MODULES => "executable_and_modules",
        EntryType::FRAMEBUFFER => "framebuffer",
        _ => "unknown",
    }
}
