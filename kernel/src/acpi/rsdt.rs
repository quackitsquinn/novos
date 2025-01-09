use core::mem;

use kserial::common::Command;
use log::info;
use x86_64::PhysAddr;

use crate::memory;

use super::sdt::SystemDescriptionTable;

/// The Root System Description Table (RSDT) is a table that contains the physical addresses of all the other System Description Tables (SDTs) in the system.
/// This is not a direct representation of the RSDT, because the RSDT is variable length.
pub struct RootSystemDescriptionTable {
    pub sdt: &'static SystemDescriptionTable,
    is_64_bit: bool,
    table: &'static [u8],
}

impl RootSystemDescriptionTable {
    pub fn new(ptr: *const (), is_64_bit: bool) -> RootSystemDescriptionTable {
        let ptr = unsafe { memory::phys_to_virt(PhysAddr::new_truncate(ptr as u64)).as_ptr() };
        let sdt = unsafe { SystemDescriptionTable::new(ptr) };

        assert!(sdt.validate_signature(), "Invalid RSDT signature");
        assert!(sdt.checksum(), "Invalid RSDT checksum");

        info!("SDT: {:?}", sdt);

        let ptr_tbl_len = sdt.length as usize - core::mem::size_of::<SystemDescriptionTable>();

        if is_64_bit {
            assert_eq!(ptr_tbl_len % 8, 0, "Invalid RSDT length");
        } else {
            assert_eq!(ptr_tbl_len % 4, 0, "Invalid RSDT length");
        }

        let ptr = unsafe {
            ptr.cast::<u8>()
                .add(core::mem::size_of::<SystemDescriptionTable>())
        };
        let table = unsafe { core::slice::from_raw_parts(ptr as *const u8, ptr_tbl_len) };
        Command::SendFile("ACPITableData.bin", table).send();

        info!(
            "Found RSDT with {} entries",
            if is_64_bit {
                table.len() / 8
            } else {
                table.len() / 4
            }
        );
        RootSystemDescriptionTable {
            sdt,
            is_64_bit,
            table,
        }
    }

    pub fn get_table(&self, index: usize) -> *const () {
        if self.is_64_bit {
            let bytes = self
                .table
                .chunks_exact(mem::size_of::<usize>())
                .nth(index)
                .unwrap();
            let ptr = bytes.as_ptr() as *const u64;
            unsafe { *ptr as *const () }
        } else {
            let bytes = self
                .table
                .chunks_exact(mem::size_of::<u32>())
                .nth(index)
                .unwrap();
            let ptr = bytes.as_ptr() as *const u32;
            unsafe { *ptr as *const () }
        }
    }

    pub fn get_table_count(&self) -> usize {
        if self.is_64_bit {
            self.table.len() / 8
        } else {
            self.table.len() / 4
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = *const ()> + '_ {
        (0..self.get_table_count()).map(move |i| self.get_table(i))
    }
}
