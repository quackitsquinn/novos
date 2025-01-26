use limine::request::RsdpRequest;
use rsdp::find_rsdp;

use crate::{declare_module, memory::paging::phys::phys_mem::MapError};

mod phys_table;
mod rsdp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ACPIInitError {
    #[error("RSDP not present")]
    RSDPNotPresent,
    #[error("Mapping error {0}")]
    MappingError(#[from] MapError),
}

fn init() -> Result<(), ACPIInitError> {
    find_rsdp()?;
    Ok(())
}

declare_module!("ACPI", init, ACPIInitError);
