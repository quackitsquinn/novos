use core::convert::Infallible;

use cake::limine::{
    paging::Mode,
    request::{
        ExecutableAddressRequest, ExecutableFileRequest, FramebufferRequest, HhdmRequest,
        MemoryMapRequest, MpRequest, PagingModeRequest, RsdpRequest,
    },
    response::ExecutableAddressResponse,
};
use cake::LimineRequest;
use spin::Once;

use crate::{
    declare_module,
    display::req_data::FramebufferInfo,
    elf::req_data::KernelElf,
    memory::req_data::MemoryMap,
    mp::ApplicationCores,
};

#[used]
pub static PHYSICAL_MEMORY_OFFSET_REQUEST: HhdmRequest = HhdmRequest::new();
pub static PHYSICAL_MEMORY_OFFSET: Once<u64> = Once::new();

pub static RSDP_ADDRESS_REQUEST: RsdpRequest = RsdpRequest::new();
pub static RSDP_ADDRESS: Once<Option<usize>> = Once::new();

#[used]
pub static MEMORY_MAP: LimineRequest<MemoryMapRequest, MemoryMap> =
    LimineRequest::new(MemoryMapRequest::new());

#[used]
pub static PAGING_MODE_REQUEST: PagingModeRequest =
    PagingModeRequest::new().with_mode(Mode::FOUR_LEVEL);

#[used]
pub static KERNEL_ELF: LimineRequest<ExecutableFileRequest, KernelElf> =
    LimineRequest::new(ExecutableFileRequest::new());

pub static FRAMEBUFFER: LimineRequest<FramebufferRequest, FramebufferInfo> =
    LimineRequest::new(FramebufferRequest::new());

pub static MP_INFO: LimineRequest<MpRequest, ApplicationCores> =
    LimineRequest::new(MpRequest::new());

#[used]
pub static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();
pub static EXECUTABLE_ADDRESS: Once<&'static ExecutableAddressResponse> = Once::new();

pub fn init() -> Result<(), Infallible> {
    let offset = PHYSICAL_MEMORY_OFFSET_REQUEST
        .get_response()
        .unwrap()
        .offset();
    PHYSICAL_MEMORY_OFFSET.call_once(|| offset);

    MEMORY_MAP.init(MemoryMap::new);

    KERNEL_ELF.init(KernelElf::new);

    FRAMEBUFFER.init(FramebufferInfo::new);

    MP_INFO.init(ApplicationCores::new);

    let exec_addr = EXECUTABLE_ADDRESS_REQUEST.get_response().unwrap();
    EXECUTABLE_ADDRESS.call_once(|| exec_addr);

    RSDP_ADDRESS.call_once(|| RSDP_ADDRESS_REQUEST.get_response().map(|r| r.address()));
    Ok(())
}

declare_module!("requests", init);
