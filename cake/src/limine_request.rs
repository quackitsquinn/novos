use core::sync::atomic::AtomicBool;

use spin::Once;

use crate::{ResourceGuard, resource::ResourceMutex};

pub trait RawLimineRequest<'a> {
    type Response;
    fn is_present(&self) -> bool;
    fn get_response(&'a self) -> Option<&'a Self::Response>;
    fn get_response_mut(&'a mut self) -> Option<&'a mut Self::Response>;
}

macro_rules! impl_lim_req {
    ($request_type:ident => $response_type:ident) => {
        impl<'a> RawLimineRequest<'a> for limine::request::$request_type {
            type Response = limine::response::$response_type;

            fn is_present(&self) -> bool {
                self.get_response().is_some()
            }

            fn get_response(&'a self) -> Option<&'a Self::Response> {
                self.get_response()
            }

            fn get_response_mut(&'a mut self) -> Option<&'a mut Self::Response> {
                (self as &mut limine::request::$request_type).get_response_mut()
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
    MpRequest => MpResponse,
}

/// A Limine request paired with kernel data initialized from the Limine response.
pub struct LimineRequest<'a, LimineType: RawLimineRequest<'a>, KernelType: 'static> {
    limine_request: ResourceMutex<LimineType>,
    pub kernel_data: Once<KernelType>,
    _phantom: core::marker::PhantomData<&'a ()>,
}

/// A guard that provides access to the Limine response data.
pub type LimineData<'a, L> = ResourceGuard<'a, L>;

impl<'a, L, K> LimineRequest<'a, L, K>
where
    L: RawLimineRequest<'a>,
{
    pub const fn new(new: L) -> Self {
        Self {
            limine_request: ResourceMutex::new(new).with_validator(requests_active),
            kernel_data: Once::new(),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Initializes the kernel data using the provided function, which is given access to the Limine response data.
    pub fn init(&'static self, data: impl FnOnce(ResourceGuard<'_, L::Response>) -> K) {
        if REQUEST_TERMINATE.load(core::sync::atomic::Ordering::SeqCst) {
            panic!("LimineRequest init called after requests terminated");
        }

        let request = self.limine_request.lock();
        if !request.is_present() {
            panic!("Limine request not present");
        }

        drop(request);
        self.kernel_data.call_once(|| data(self.get_limine()));
    }

    /// Returns a reference to the kernel data. This will panic if the data has not been initialized yet.
    pub fn get(&'static self) -> &'static K {
        self.kernel_data
            .get()
            .expect("LimineRequest kernel data not initialized")
    }

    /// Returns a lock guard to the Limine response data.
    pub fn get_limine(&'a self) -> ResourceGuard<'a, L::Response> {
        self.limine_request
            .lock_map(|t| t.get_response_mut().expect("response not present"))
    }
}

static REQUEST_TERMINATE: AtomicBool = AtomicBool::new(false);

/// Terminates all limine requests. After this is called, no further calls to `init` on any
/// `LimineRequest` will be allowed. This should be called once the kernel has finished
/// remapping and requests may be dangling pointers.
pub unsafe fn terminate_requests() {
    REQUEST_TERMINATE.store(true, core::sync::atomic::Ordering::SeqCst);
}

/// Returns true if limine requests have been terminated.
pub fn requests_terminated() -> bool {
    REQUEST_TERMINATE.load(core::sync::atomic::Ordering::SeqCst)
}

/// Returns true if limine requests are still active (i.e., have not been terminated).
pub fn requests_active() -> bool {
    !requests_terminated()
}
