use core::sync::atomic::AtomicBool;

use spin::Once;

pub trait RawLimineRequest {
    type Response;
    fn is_present(&'static self) -> bool;
    fn get_response<'a>(&'static self) -> Option<&'a Self::Response>;
}

macro_rules! impl_lim_req {
    ($request_type:ident => $response_type:ident) => {
        impl RawLimineRequest for limine::request::$request_type {
            type Response = limine::response::$response_type;

            fn is_present(&'static self) -> bool {
                self.get_response().is_some()
            }

            fn get_response<'a>(&'static self) -> Option<&'a Self::Response> {
                self.get_response()
            }
        }
    };

    ($($request_type:ident => $response_type:ident),* $(,)?) => {
        $(
            impl_lim_req!($request_type => $response_type);
        )*
    };
}

impl_lim_req! {
    FramebufferRequest => FramebufferResponse,
    MemoryMapRequest => MemoryMapResponse,
    ExecutableFileRequest => ExecutableFileResponse,
}

pub struct LimineRequest<LimineType: RawLimineRequest, KernelType: 'static> {
    limine_request: Option<LimineType>,
    pub kernel_data: Once<KernelType>,
}

impl<L, K> LimineRequest<L, K>
where
    L: RawLimineRequest,
{
    pub const fn new(new: L) -> Self {
        Self {
            limine_request: Some(new),
            kernel_data: Once::new(),
        }
    }

    pub fn init(&'static self, data: impl FnOnce(&L::Response) -> K) {
        if REQUEST_TERMINATE.load(core::sync::atomic::Ordering::SeqCst) {
            panic!("LimineRequest init called after requests terminated");
        }
        if !self.limine_request.as_ref().unwrap().is_present() {
            panic!("Limine request not present");
        }

        let response = self
            .limine_request
            .as_ref()
            .unwrap()
            .get_response()
            .expect("Limine response not present");

        self.kernel_data.call_once(|| data(response));
    }

    pub fn get(&'static self) -> &'static K {
        self.kernel_data
            .get()
            .expect("LimineRequest kernel data not initialized")
    }
}

static REQUEST_TERMINATE: AtomicBool = AtomicBool::new(false);

/// Terminates all limine requests. After this is called, no further calls to `init` on any
/// `LimineRequest` will be allowed. This should be called once the kernel has finished
/// remapping and requests may be dangling pointers.
pub unsafe fn terminate_requests() {
    REQUEST_TERMINATE.store(true, core::sync::atomic::Ordering::SeqCst);
}

pub fn requests_terminated() -> bool {
    REQUEST_TERMINATE.load(core::sync::atomic::Ordering::SeqCst)
}
