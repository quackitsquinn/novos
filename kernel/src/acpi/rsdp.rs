use alloc::{str, string::String};
use limine::request::RsdpRequest;
use log::info;
use x86_64::PhysAddr;

use crate::acpi::phys_table::PhysicalTable;

use super::ACPIInitError;

#[repr(packed, C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootSystemDescriptionPointer {
    sig: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    ext_checksum: u8,
    reserved: [u8; 3],
}

impl RootSystemDescriptionPointer {
    pub unsafe fn new(ptr: *const ()) -> &'static RootSystemDescriptionPointer {
        let byte_ptr = ptr.cast::<u8>();
        // Get to the revision field
        let rev_ptr = unsafe { byte_ptr.add(15) };
        let revision = unsafe { *rev_ptr };

        // We can now safely cast the pointer to the RSDP
        let ptr = unsafe { &*ptr.cast::<RootSystemDescriptionPointer>() };
        info!("OEM ID: {:?}", core::str::from_utf8(&ptr.oem_id).unwrap());
        // Check the signature
        assert_eq!(
            ptr.sig,
            *b"RSD PTR ",
            "Invalid RSDP signature! Got {:?} expected {:?}",
            String::from_utf8_lossy(&ptr.sig),
            b"RSD PTR "
        );
        // Check the checksum
        let mut sum: u8 = 0;
        for i in 0..20 {
            sum = sum.wrapping_add(unsafe { *byte_ptr.add(i) });
        }
        assert_eq!(sum, 0, "Invalid RSDP checksum");

        // Check the extended checksum
        if revision > 0 {
            let mut sum: u8 = 0;
            for i in 0..size_of::<RootSystemDescriptionPointer>() as usize {
                sum = sum.wrapping_add(unsafe { *byte_ptr.add(i) });
            }
            assert_eq!(sum, 0, "Invalid RSDP extended checksum");
        }
        ptr
    }
    /// Returns the physical address of the RSDT. In (ptr, is_64_bit) format.
    pub fn get_table_ptr(&self) -> (*const (), bool) {
        if self.revision == 0 {
            (self.rsdt_address as *const (), false)
        } else {
            (self.xsdt_address as *const (), true)
        }
    }
}

static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

pub(super) fn find_rsdp() -> Result<(), ACPIInitError> {
    let rsdp = RSDP_REQUEST
        .get_response()
        .ok_or(ACPIInitError::RSDPNotPresent)?;
    // rsdp is a PHYSICAL address
    let table = unsafe { RootSystemDescriptionPointer::new(rsdp.address() as *const ()) };
    info!("RSDP found! {:?}", table);
    todo!()
}
