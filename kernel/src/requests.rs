use core::{convert::Infallible, slice};

use kelp::Elf;
use limine::{
    file::File,
    paging::Mode,
    request::{
        ExecutableAddressRequest, ExecutableFileRequest, FramebufferRequest, HhdmRequest,
        MemoryMapRequest, PagingModeRequest,
    },
    response::{
        ExecutableAddressResponse, ExecutableFileResponse, FramebufferResponse, MemoryMapResponse,
    },
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
pub static EXECUTABLE_DATA: Once<&'static [u8]> = Once::new();
pub static EXECUTABLE_ELF: Once<Elf> = Once::new();

#[used]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
pub static FRAMEBUFFERS: Once<&'static FramebufferResponse> = Once::new();

#[used]
pub static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();
pub static EXECUTABLE_ADDRESS: Once<&'static ExecutableAddressResponse> = Once::new();

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
    EXECUTABLE_DATA.call_once(|| unsafe {
        slice::from_raw_parts(exec_file.file().addr(), exec_file.file().size() as usize)
    });
    EXECUTABLE_ELF.call_once(|| {
        Elf::new(
            EXECUTABLE_DATA
                .get()
                .expect("Executable data not initialized"),
        )
        .expect("Failed to create ELF from executable data")
    });

    let fb = FRAMEBUFFER_REQUEST.get_response().unwrap();
    FRAMEBUFFERS.call_once(|| fb);

    let exec_addr = EXECUTABLE_ADDRESS_REQUEST.get_response().unwrap();
    EXECUTABLE_ADDRESS.call_once(|| exec_addr);
    Ok(())
}

declare_module!("requests", init);
