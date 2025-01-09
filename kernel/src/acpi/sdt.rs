use core::fmt::Debug;

use alloc::{format, string::String};
use log::trace;

#[repr(C, packed)]
pub struct SystemDescriptionTable {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl Debug for SystemDescriptionTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Because it's a packed struct, we have to make some of the fields local variables. I'm not entirely sure why.
        let len = self.length;
        let oem_revision = self.oem_revision;
        let creator_revision = self.creator_revision;

        let sig_valid = self.signature.iter().all(|&c| c.is_ascii());
        let oem_id_valid = self.oem_id.iter().all(|&c| c.is_ascii());
        let oem_table_id_valid = self.oem_table_id.iter().all(|&c| c.is_ascii());
        let creator_id_valid = self.creator_id.to_le_bytes().iter().all(|&c| c.is_ascii());

        let mut f = f.debug_struct("SystemDescriptionTable");
        if sig_valid {
            f.field("signature", &String::from_utf8_lossy(&self.signature));
        } else {
            f.field("signature", &format!("Invalid: {:?}", &self.signature));
        }
        f.field("length", &len)
            .field("revision", &self.revision)
            .field("checksum", &self.checksum);
        if oem_id_valid {
            f.field("oem_id", &String::from_utf8_lossy(&self.oem_id));
        } else {
            f.field("oem_id", &format!("Invalid: {:?}", &self.oem_id));
        }
        if oem_table_id_valid {
            f.field("oem_table_id", &String::from_utf8_lossy(&self.oem_table_id));
        } else {
            f.field(
                "oem_table_id",
                &format!("Invalid: {:?}", &self.oem_table_id),
            );
        }
        f.field("oem_revision", &oem_revision);
        if creator_id_valid {
            f.field(
                "creator_id",
                &String::from_utf8_lossy(&self.creator_id.to_le_bytes()),
            );
        } else {
            f.field(
                "creator_id",
                &format!("Invalid: {:?}", &self.creator_id.to_le_bytes()),
            );
        }
        f.field("creator_revision", &creator_revision).finish()
    }
}

impl SystemDescriptionTable {
    pub unsafe fn new(ptr: *const ()) -> &'static SystemDescriptionTable {
        unsafe { &*(ptr as *const SystemDescriptionTable) }
    }

    pub fn validate_signature(&self) -> bool {
        VALID_SIGNATURES.iter().any(|&sig| sig == self.signature)
    }

    pub fn checksum(&self) -> bool {
        // Checksum
        let table_bytes = unsafe {
            core::slice::from_raw_parts(
                (self as *const SystemDescriptionTable).cast::<u8>(),
                self.length as usize,
            )
        };

        let sum = table_bytes
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_add(byte));
        if sum != 0 {
            trace!("Invalid checksum for SDT");
            return false;
        }

        true
    }
}

const VALID_SIGNATURES: &'static [[u8; 4]] = &[
    *b"RSDT", *b"XSDT", *b"FACP", *b"HPET", *b"APIC", *b"MCFG", *b"SSDT", *b"BERT", *b"BGRT",
    *b"CPEP", *b"DSDT", *b"ECDT", *b"EINJ", *b"ERST", *b"FACS", *b"FPDT", *b"GTDT", *b"HEST",
    *b"MSCT", *b"MPST", *b"NFIT", *b"PCCT", *b"PHAT", *b"PMTT", *b"PSDT", *b"RASF", *b"SBST",
    *b"SDEV", *b"SLIT", *b"SRAT", *b"AEST", *b"BDAT", *b"CDIT", *b"CEDT", *b"CRAT", *b"CSRT",
    *b"DBGP", *b"DBG2", *b"DMAR", *b"DRTM", *b"ETDT", *b"IBFT", *b"IORT", *b"IVRS", *b"LPIT",
    *b"MCHI", *b"MPAM", *b"MSDM", *b"PRMT", *b"RGRT", *b"SDEI", *b"SLIC", *b"SPCR", *b"SPMI",
    *b"STAO", *b"SVKL", *b"TCPA", *b"TPM2", *b"UEFI", *b"WAET", *b"WDAT", *b"WDRT", *b"WPBT",
    *b"WSMT", *b"XENV",
];
