use core::convert::Infallible;

use limine::paging::Mode;
use spin::Once;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
    VirtAddr,
};

use crate::{declare_module, util::OnceMutex};

pub mod phys;
pub mod virt;

#[used]
static PAGE_TABLE_REQUEST: limine::request::PagingModeRequest =
    limine::request::PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
static MEMORY_OFFSET_REQUEST: limine::request::HhdmRequest = limine::request::HhdmRequest::new();

#[used]
static MEMORY_MAP_REQUEST: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

pub static MEMORY_OFFSET: Once<u64> = Once::new();
pub static OFFSET_PAGE_TABLE: OnceMutex<OffsetPageTable> = OnceMutex::uninitialized();

declare_module!("paging", init);

fn init() -> Result<(), Infallible> {
    let off = MEMORY_OFFSET_REQUEST.get_response().unwrap().offset();
    MEMORY_OFFSET.call_once(|| off);
    let cr3 = Cr3::read();
    let page_table = unsafe { &mut *((cr3.0.start_address().as_u64() + off) as *mut PageTable) };
    OFFSET_PAGE_TABLE.init(unsafe { OffsetPageTable::new(page_table, VirtAddr::new(off)) });
    phys::MODULE.init();
    Ok(())
}
