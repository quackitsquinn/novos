use core::convert::Infallible;

use limine::paging::Mode;
use spin::Once;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{MappedPageTable, OffsetPageTable, PageTable},
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

/// Defines various memory map constants used by the kernel.
///
/// KERNEL_* = Kernel memory
///
/// KERNEL_HEAP_* = Kernel heap memory
///
/// KERNEL_PHYS_MAP_* = Kernel misc memory (e.g virtual/physical memory mapping)
///
/// KERNEL_BINARY = Kernel binary memory
///
/// HIGHER_HALF_START = Start of the higher half of the kernel memory
pub mod map {
    use x86_64::VirtAddr;

    macro_rules! define_map {
        ($name:ident, $start:expr, $size:expr) => {
            const _: () = {
                assert!($start < MAX_VIRT_ADDR.as_u64());
                assert!($size < MAX_VIRT_ADDR.as_u64());
                assert!($start + $size < MAX_VIRT_ADDR.as_u64());
                ()
            };
            paste::paste! {
                pub const [<$name _START>]: ::x86_64::VirtAddr = ::x86_64::VirtAddr::new_truncate($start);
                pub const [<$name _SIZE>]: u64 = $size;
                pub const [<$name _END>]: ::x86_64::VirtAddr = ::x86_64::VirtAddr::new_truncate($start + $size);
            }
        };
    }

    // Taken from linker script
    pub const KERNEL_BINARY: u64 = 0xFFFF_FFFF_8000_0000;
    pub const HIGHER_HALF_START: u64 = 0xFFFF_8000_0000;
    pub const MAX_VIRT_ADDR: VirtAddr = VirtAddr::new_truncate(u64::MAX);

    pub const KERNEL_START: u64 = HIGHER_HALF_START + 0x1000_0000;

    define_map!(KERNEL_HEAP, KERNEL_START, 0xA0_0000); // 10MB
    define_map!(KERNEL_PHYS_MAP, KERNEL_HEAP_END.as_u64(), 0x1000_0000); // 256MB
}
