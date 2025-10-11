//! ACPI (Advanced Configuration and Power Interface) support. Contains logic for parsing and interacting with ACPI tables.
//! This includes methods for accessing various ACPI tables and their entries (see the [mapped_table] module).
use core::{mem, ops::Deref, ptr::read_unaligned};

use acpi::{rsdp::Rsdp, sdt::Signature, AcpiError};
use alloc::collections::btree_map::BTreeMap;
use cake::log::{info, warn};
use cake::spin::{Mutex, MutexGuard, Once};
use cake::Owned;
pub use mapped_table::MappedTable;
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

use crate::{acpi::sdt::TableHeader, declare_module, memory::paging::phys::phys_mem::map_address};

pub mod mapped_table;
pub mod sdt;

/// The Root System Description Pointer (RSDP) structure.
pub static RSDP: Once<Owned<Rsdp>> = Once::new();

/// A locked ACPI table. This prevents any concurrent access.
pub type AcpiTableLock = MutexGuard<'static, TableHeader<'static>>;

static ACPI_TABLES: Once<BTreeMap<u32, Mutex<TableHeader<'static>>>> = Once::new();

fn init() -> Result<(), AcpiError> {
    let rsdp_addr = *crate::requests::RSDP_ADDRESS
        .get()
        .expect("RSDP address not set")
        .as_ref()
        .ok_or(AcpiError::NoValidRsdp)?;

    let rsdp_table = map_address(
        PhysAddr::new(rsdp_addr as u64),
        size_of::<Rsdp>() as u64,
        PageTableFlags::PRESENT,
    )
    .expect("Failed to map RSDP");

    let rdsp = unsafe { Owned::new(&mut *(rsdp_table.ptr() as *mut Rsdp)) };
    rdsp.validate()?;
    RSDP.call_once(|| rdsp);

    let rsdp = RSDP.get().unwrap().deref();

    info!("ACPI Version: {}", rsdp.revision);

    let sdt_table: u64;
    let ptr_len: usize;
    if rsdp.revision == 0 {
        let rdst = rsdp.rsdt_address;
        info!("ACPI 1.0 RSDT Address: {:#x}", rdst);
        sdt_table = rdst as u64;
        ptr_len = 4;
    } else {
        let xsdt = rsdp.xsdt_address;
        info!("ACPI 2.0+ XSDT Address: {:#x}", xsdt);
        sdt_table = xsdt;
        ptr_len = 8;
    }

    let sdt = unsafe { TableHeader::new(PhysAddr::new(sdt_table)) };

    let entries = (sdt.header().length as usize - mem::size_of::<acpi::sdt::SdtHeader>()) / ptr_len;

    let len = sdt.header().length as u64;

    info!(
        "ACPI SDT at {:#x}, length: {}, entries: {}",
        sdt_table, len, entries
    );

    let table = sdt.table_ptr();

    let mut tables = BTreeMap::new();

    for off in (0..entries).map(|i| i * ptr_len) {
        let entry_addr = unsafe {
            if ptr_len == 4 {
                table.add(off).cast::<u32>().read_unaligned() as u64
            } else {
                table.add(off).cast::<u64>().read_unaligned() as u64
            }
        };
        info!("  Entry {}: {:#x}", off / ptr_len, entry_addr);
        let entry = unsafe { TableHeader::new(PhysAddr::new(entry_addr)) };
        if let Err(e) = entry.validate(entry.header().signature) {
            warn!(
                "ACPI table at {:#x} has invalid signature: {:?}",
                entry_addr, e
            );
            continue;
        }
        info!("    Signature: {}", entry.header().signature,);
        let key = table_key(entry.header().signature);
        tables.insert(key, Mutex::new(entry));
    }

    ACPI_TABLES.call_once(|| tables);

    Ok(())
}

/// Converts a table signature to a key for the BTreeMap.
fn table_key(sig: Signature) -> u32 {
    unsafe { read_unaligned(&sig as *const _ as *const u32) }
}

/// Returns a locked ACPI table with the given signature.
pub fn get_table(signature: Signature) -> Option<AcpiTableLock> {
    let tables = ACPI_TABLES.get().expect("ACPI tables not initialized");
    let key = table_key(signature);
    tables.get(&key).map(|t| t.try_lock()).flatten()
}

declare_module!("ACPI", init, AcpiError);
