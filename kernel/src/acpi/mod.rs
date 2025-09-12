mod mapped_table;
use core::{mem, ops::Deref};

use acpi::{rsdp::Rsdp, AcpiError};
use log::info;
pub use mapped_table::MappedTable;
use spin::Once;
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

use crate::{declare_module, memory::paging::phys::phys_mem::map_address, util::Owned};

pub static RSDP: Once<Owned<Rsdp>> = Once::new();

pub fn init() -> Result<(), AcpiError> {
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
    if rsdp.revision == 0 {}

    Ok(())
}

declare_module!("ACPI", init, AcpiError);
