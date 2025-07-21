use core::slice;

use cake::Once;
use limine::{
    file::File,
    paging::Mode,
    request::{HhdmRequest, MemoryMapRequest, ModuleRequest, PagingModeRequest, StackSizeRequest},
    response::MemoryMapResponse,
};

use crate::STACK_SIZE;

#[used]
pub static PAGING_MODE_REQUEST: PagingModeRequest =
    PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
pub static PHYSICAL_MEMORY_OFFSET_REQUEST: HhdmRequest = HhdmRequest::new();
pub static PHYSICAL_MEMORY_OFFSET: Once<u64> = Once::new();

#[used]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
pub static MEMORY_MAP: Once<&'static MemoryMapResponse> = Once::new();

#[used]
pub static STACK_SIZE_REQUEST: StackSizeRequest =
    StackSizeRequest::new().with_size(STACK_SIZE as u64);

#[used]
pub static MODULES: ModuleRequest = ModuleRequest::new();
pub static KERNEL_FILE: Once<&'static [u8]> = Once::new();

pub fn load() {
    let offset = PHYSICAL_MEMORY_OFFSET_REQUEST
        .get_response()
        .unwrap()
        .offset();
    PHYSICAL_MEMORY_OFFSET.call_once(|| offset);
    let mmap = MEMORY_MAP_REQUEST.get_response().unwrap();
    MEMORY_MAP.call_once(|| mmap);
    if STACK_SIZE_REQUEST.get_response().is_none() {
        panic!("Stack size request failed!")
    }

    MODULES
        .get_response()
        .expect("Module request failed!")
        .modules()
        .iter()
        .find(|m| {
            if let Ok(name) = m.path().to_str() {
                if name.contains("kernel.bin") {
                    return true;
                }
            }
            return false;
        })
        .map(|m| {
            KERNEL_FILE.call_once(|| unsafe { slice::from_raw_parts(m.addr(), m.size() as usize) });
        })
        .expect("Kernel module not found in module request response");
}
