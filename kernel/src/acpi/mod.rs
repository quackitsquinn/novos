use core::mem;

use alloc::vec::Vec;
use limine::request::RsdpRequest;
use log::info;
use rsdp::RootSystemDescriptionPointer;
use rsdt::RootSystemDescriptionTable;
use spin::Once;
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

use crate::{
    memory::{
        self,
        paging::phys::{PhysicalMap, PhysicalMapResult},
    },
    println,
};

mod rsdp;
mod rsdt;
mod sdt;

#[used]
static RSDP_PTR_REQUEST: RsdpRequest = RsdpRequest::new();

pub static RSDP: Once<&rsdp::RootSystemDescriptionPointer> = Once::new();
pub static TABLES: Once<Vec<&sdt::SystemDescriptionTable>> = Once::new();

pub fn init() {
    let rsdp = RSDP_PTR_REQUEST.get_response().unwrap();
    info!(
        "RSDP found at 0x{:p} (revision: {})",
        rsdp.address(),
        rsdp.revision()
    );
    //let phys = PhysAddr::new_truncate(rsdp.address() as u64);
    //let virt = unsafe { memory::phys_to_virt(phys) };
    //TODO: virt might need to be added to the page table to allow for reading
    let rsdp = unsafe { RootSystemDescriptionPointer::new(rsdp.address()) };

    let ptr = rsdp.get_table_ptr();
    let root_table = RootSystemDescriptionTable::new(ptr.0, ptr.1);
    let mut mappings: Vec<PhysicalMapResult> = Vec::new();
    let mut tables: Vec<&sdt::SystemDescriptionTable> = Vec::new();
    for table in root_table
        .iter()
        .map(|tbl| PhysAddr::new_truncate(tbl as u64))
    {
        let mut addr = 0;
        if !mappings
            .iter()
            .any(|m| m.contains(table, mem::size_of::<sdt::SystemDescriptionTable>() as u64))
        {
            let map = unsafe {
                PhysicalMap::new(
                    table,
                    mem::size_of::<sdt::SystemDescriptionTable>(),
                    PageTableFlags::PRESENT,
                )
                .map()
            }
            .expect("Could not map RSDT entry");
            addr = map.virt().as_u64();
            mappings.push(map);
        } else {
            mappings.iter().for_each(|m| {
                if m.contains(table, mem::size_of::<sdt::SystemDescriptionTable>() as u64) {
                    addr = m.virt().as_u64()
                }
            });
        }
        if addr != 0 {
            let sdt = unsafe { sdt::SystemDescriptionTable::new(addr as *const ()) };
            println!("SDT: {:#?}", sdt);
            tables.push(sdt);
        }
    }
    println!("Found {} tables: ", tables.len());
    for table in tables {
        println!("{:?}", table);
    }

    RSDP.call_once(|| rsdp);
}
