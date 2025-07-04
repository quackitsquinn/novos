use limine::{
    paging::Mode,
    request::{HhdmRequest, MemoryMapRequest, PagingModeRequest},
    response::MemoryMapResponse,
};
use spin::Once;

#[used]
pub static PAGING_MODE_REQUEST: PagingModeRequest =
    PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
pub static PHYSICAL_MEMORY_OFFSET_REQUEST: HhdmRequest = HhdmRequest::new();
pub static PHYSICAL_MEMORY_OFFSET: Once<u64> = Once::new();

#[used]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
pub static MEMORY_MAP: Once<&'static MemoryMapResponse> = Once::new();

pub fn load() {
    let offset = PHYSICAL_MEMORY_OFFSET_REQUEST
        .get_response()
        .unwrap()
        .offset();
    PHYSICAL_MEMORY_OFFSET.call_once(|| offset);
    let mmap = MEMORY_MAP_REQUEST.get_response().unwrap();
    MEMORY_MAP.call_once(|| mmap);
}
