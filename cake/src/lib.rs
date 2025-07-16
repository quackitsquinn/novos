//! That's a piece of cake!
//! This crate provides a set of utilities and abstractions for kernel development in Rust.

#![no_std]

mod module;
mod once_mutex;

pub use bitflags::bitflags;
pub use log::{self, debug, error, info, trace, warn};
pub use module::KernelModule;
pub use once_mutex::OnceMutex;
pub use spin::{self, Mutex, Once, RwLock};

static CALLER_INSTRUCTION_POINTER_FN: Once<fn() -> usize> = Once::new();
static CALLER_INSTRUCTION_POINTER_NAME_RESOLVER: Once<fn(usize) -> Option<&'static str>> =
    Once::new();

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

#[inline(never)]
fn get_caller_rip() -> Option<*const ()> {
    let func = CALLER_INSTRUCTION_POINTER_FN.get()?;
    Some(func() as *const ())
}

fn resolve_symbol(addr: *const ()) -> Option<&'static str> {
    let resolver = CALLER_INSTRUCTION_POINTER_NAME_RESOLVER.get()?;
    resolver(addr as usize)
}
