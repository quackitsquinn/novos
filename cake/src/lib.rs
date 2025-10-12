#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, feature(thread_id_value))]
#![feature(debug_closure_helpers)]

mod fuse;
mod limine_request;
mod module;
mod oncemut;
mod oncerw;
mod owned;
mod resource;

/* Crate Exports */
pub use self::limine_request::{
    LimineData, LimineRequest, RawLimineRequest, requests_terminated, terminate_requests,
};
pub use fuse::Fuse;
pub use module::KernelModule;
pub use oncemut::OnceMutex;
pub use oncerw::{OnceRwLock, OnceRwReadGuard, OnceRwWriteGuard};
pub use owned::Owned;
use raw_cpuid::CpuId;
pub use resource::{ResourceGuard, ResourceMutex};
use spin::Once;

pub use limine;
pub use log;
pub use spin;

static CALLER_INSTRUCTION_POINTER_FN: Once<fn() -> usize> = Once::new();
static CALLER_INSTRUCTION_POINTER_NAME_RESOLVER: Once<fn(usize) -> Option<&'static str>> =
    Once::new();
static MULTITHREADED: Once<bool> = Once::new();

/// Sets the function that will be used to get the instruction pointer of the caller.
/// This function should return the caller 2 levels up the stack.
///
/// Any inaccuracies will not cause U.B. because this is treated as a heuristic.
///
/// The stack will look like this:
/// - caller
/// - wrapper function
/// - instruction pointer function (`f`)
pub fn set_caller_instruction_pointer_fn(f: fn() -> usize) {
    CALLER_INSTRUCTION_POINTER_FN.call_once(|| f);
}

/// Sets the function that will be used to resolve the instruction pointer to a symbol name.
/// This function should return the name of the symbol at the given instruction pointer.
/// If the symbol cannot be found, it should return `None`.
///
/// Similar to the above, this is treated as a heuristic and inaccuracies will not cause U.B.
pub fn set_caller_instruction_pointer_name_resolver(f: fn(usize) -> Option<&'static str>) {
    CALLER_INSTRUCTION_POINTER_NAME_RESOLVER.call_once(|| f);
}

/// Returns true if the kernel is running in a multithreaded environment (i.e., with multiple cores).
/// This is set automatically to single-threaded until this function is called.
pub fn set_multithreaded(multithreaded: bool) {
    MULTITHREADED.call_once(|| multithreaded);
}

/// Returns true if the kernel is running in a multithreaded environment (i.e., with multiple cores).
pub(crate) fn is_multithreaded() -> bool {
    *MULTITHREADED.get().unwrap_or(&false)
}

#[inline(never)]
fn get_caller_rip_1_up() -> Option<*const ()> {
    let func = CALLER_INSTRUCTION_POINTER_FN.get()?;
    Some(func() as *const ())
}

fn resolve_symbol(addr: *const ()) -> Option<&'static str> {
    let resolver = CALLER_INSTRUCTION_POINTER_NAME_RESOLVER.get()?;
    resolver(addr as usize)
}

mod _macro {
    macro_rules! get_caller_rip_2_up {
        () => {
            $crate::CALLER_INSTRUCTION_POINTER_FN
                .get()
                .map(|f| f() as *const ())
        };
    }
    pub(crate) use get_caller_rip_2_up;
}

pub(crate) use _macro::get_caller_rip_2_up;

#[allow(unreachable_code)]
pub fn core_id() -> u64 {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    return CpuId::with_cpuid_reader(raw_cpuid::CpuIdReaderNative)
        .get_feature_info()
        .map_or(0, |finfo| finfo.initial_local_apic_id() as u64);
    #[cfg(not(target_arch = "x86_64"))]
    return 0;
    #[cfg(any(test, feature = "std"))]
    return std::thread::current().id().as_u64().into();
}

#[cfg(test)]
mod test_log {
    use ctor::ctor;

    #[ctor]
    fn log_setup() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
}
