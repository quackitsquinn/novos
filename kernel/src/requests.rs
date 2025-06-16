use core::convert::Infallible;

use limine::{
    file::File,
    paging::Mode,
    request::{
        ExecutableFileRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest, PagingModeRequest,
    },
    response::{ExecutableFileResponse, FramebufferResponse, MemoryMapResponse},
};
use spin::Once;

use crate::declare_module;

#[used]
pub static PHYSICAL_MEMORY_OFFSET_REQUEST: HhdmRequest = HhdmRequest::new();
pub static PHYSICAL_MEMORY_OFFSET: Once<u64> = Once::new();

#[used]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
pub static MEMORY_MAP: Once<&'static MemoryMapResponse> = Once::new();

#[used]
pub static PAGING_MODE_REQUEST: PagingModeRequest =
    PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
pub static EXECUTABLE_FILE_REQUEST: ExecutableFileRequest = ExecutableFileRequest::new();
pub static EXECUTABLE_FILE: Once<&'static ExecutableFileResponse> = Once::new();

#[used]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
pub static FRAMEBUFFERS: Once<&'static FramebufferResponse> = Once::new();

pub fn init() -> Result<(), Infallible> {
    let offset = PHYSICAL_MEMORY_OFFSET_REQUEST
        .get_response()
        .unwrap()
        .offset();
    PHYSICAL_MEMORY_OFFSET.call_once(|| offset);

    let mmap = MEMORY_MAP_REQUEST.get_response().unwrap();
    MEMORY_MAP.call_once(|| mmap);

    let exec_file = EXECUTABLE_FILE_REQUEST.get_response().unwrap();
    EXECUTABLE_FILE.call_once(|| exec_file);

    let fb = FRAMEBUFFER_REQUEST.get_response().unwrap();
    FRAMEBUFFERS.call_once(|| fb);
    Ok(())
}

declare_module!("requests", init);
