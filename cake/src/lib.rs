#![no_std]

mod limine_request;
mod module;
mod oncemut;
mod owned;
mod resource;

pub use self::limine_request::{
    LimineData, LimineRequest, RawLimineRequest, requests_terminated, terminate_requests,
};
pub use module::KernelModule;
pub use oncemut::OnceMutex;
pub use owned::Owned;
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
